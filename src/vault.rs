//! The key that encrypts the history, and encryption with it.
//!
//! The key is 32 random bytes. The history file keeps it sealed with DPAPI
//! (Windows encryption tied to the user's account), so only that account on
//! that PC can use it. Each item and each image file is encrypted with
//! AES-256-GCM and a fresh random nonce, and bound to what it is and its id,
//! so a row or a file can't be swapped for another. Systems without DPAPI
//! keep the history plain.

use anyhow::{anyhow, bail, Context, Result};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};

/// The first byte of an encrypted value. A plain item starts with `{`, and a
/// later format would get another number.
const FORMAT: u8 = 1;
const TAG_LEN: usize = 16;

pub struct Vault {
    key: LessSafeKey,
    random: SystemRandom,
}

impl Vault {
    /// A new random key, with its sealed form to store. `None` on systems
    /// without DPAPI, where the history stays plain.
    pub fn create() -> Result<Option<(Vault, Vec<u8>)>> {
        let random = SystemRandom::new();
        let mut raw = [0u8; 32];
        random
            .fill(&mut raw)
            .map_err(|_| anyhow!("no random numbers for a key"))?;
        let Some(sealed) = crate::secure::protect(&raw)? else {
            return Ok(None);
        };
        Ok(Some((Self::from_raw(&raw)?, sealed)))
    }

    /// The key from its sealed form. Fails on another Windows account or PC.
    pub fn open(sealed: &[u8]) -> Result<Vault> {
        Self::from_raw(&crate::secure::unprotect(sealed)?)
    }

    fn from_raw(raw: &[u8]) -> Result<Vault> {
        let key = UnboundKey::new(&AES_256_GCM, raw)
            .map_err(|_| anyhow!("the key has the wrong length"))?;
        Ok(Vault {
            key: LessSafeKey::new(key),
            random: SystemRandom::new(),
        })
    }

    /// `plain` encrypted and bound to `context`, which says what it is.
    pub fn seal(&self, context: &[u8], plain: &[u8]) -> Result<Vec<u8>> {
        let mut nonce = [0u8; NONCE_LEN];
        self.random
            .fill(&mut nonce)
            .map_err(|_| anyhow!("no random numbers for encryption"))?;
        let mut body = plain.to_vec();
        self.key
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(context),
                &mut body,
            )
            .map_err(|_| anyhow!("encrypting failed"))?;
        let mut out = Vec::with_capacity(1 + NONCE_LEN + body.len());
        out.push(FORMAT);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&body);
        Ok(out)
    }

    /// What `seal` was given, if `sealed` is intact and bound to `context`.
    pub fn unseal(&self, context: &[u8], sealed: &[u8]) -> Result<Vec<u8>> {
        let (&format, rest) = sealed.split_first().context("an empty encrypted value")?;
        if format != FORMAT {
            bail!("an encrypted value in an unknown format ({format})");
        }
        if rest.len() < NONCE_LEN + TAG_LEN {
            bail!("an encrypted value that is too short");
        }
        let (nonce, body) = rest.split_at(NONCE_LEN);
        let nonce = Nonce::try_assume_unique_for_key(nonce).map_err(|_| anyhow!("a bad nonce"))?;
        let mut body = body.to_vec();
        let plain = self
            .key
            .open_in_place(nonce, Aad::from(context), &mut body)
            .map_err(|_| anyhow!("an encrypted value is damaged or belongs elsewhere"))?;
        Ok(plain.to_vec())
    }
}

/// Whether `bytes` came from `seal`: a plain item starts with `{` instead.
pub fn is_sealed(bytes: &[u8]) -> bool {
    bytes.first() == Some(&FORMAT)
}

/// What an item row is bound to.
pub fn item_context(id: u64) -> Vec<u8> {
    [b"item ".as_slice(), &id.to_le_bytes()].concat()
}

/// What a text's formatting row is bound to.
pub fn formats_context(id: u64) -> Vec<u8> {
    [b"formats ".as_slice(), &id.to_le_bytes()].concat()
}

/// What an image file is bound to.
pub fn image_context(id: u64) -> Vec<u8> {
    [b"image ".as_slice(), &id.to_le_bytes()].concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_vault() -> Vault {
        Vault::from_raw(&[9; 32]).unwrap()
    }

    #[test]
    fn a_sealed_value_opens_again_and_hides_its_text() {
        let vault = test_vault();
        let sealed = vault.seal(&item_context(1), b"my secret copy").unwrap();
        assert!(is_sealed(&sealed));
        assert!(!sealed.windows(6).any(|w| w == b"secret"));
        assert_eq!(
            vault.unseal(&item_context(1), &sealed).unwrap(),
            b"my secret copy"
        );
        // The same text never seals the same way twice.
        assert_ne!(
            sealed,
            vault.seal(&item_context(1), b"my secret copy").unwrap()
        );
    }

    #[test]
    fn a_changed_or_moved_value_is_refused() {
        let vault = test_vault();
        let sealed = vault.seal(&item_context(1), b"my secret copy").unwrap();
        assert!(
            vault.unseal(&item_context(2), &sealed).is_err(),
            "another item"
        );
        assert!(
            vault.unseal(&image_context(1), &sealed).is_err(),
            "an image"
        );
        let mut changed = sealed.clone();
        *changed.last_mut().unwrap() ^= 1;
        assert!(
            vault.unseal(&item_context(1), &changed).is_err(),
            "a changed byte"
        );
        let other = Vault::from_raw(&[8; 32]).unwrap();
        assert!(
            other.unseal(&item_context(1), &sealed).is_err(),
            "another key"
        );
        assert!(!is_sealed(br#"{"id":1}"#));
    }

    #[cfg(windows)]
    #[test]
    fn the_key_is_sealed_for_this_account() {
        let (vault, sealed) = Vault::create().unwrap().expect("DPAPI on Windows");
        let reopened = Vault::open(&sealed).unwrap();
        let value = vault.seal(&item_context(3), b"copy").unwrap();
        assert_eq!(reopened.unseal(&item_context(3), &value).unwrap(), b"copy");
        assert!(Vault::open(b"not a sealed key").is_err());
    }
}
