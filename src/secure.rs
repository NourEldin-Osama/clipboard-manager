//! Keeps secrets such as API keys unreadable in the settings file. On Windows
//! they are encrypted with DPAPI, so only the same user on the same PC can
//! read them back. Other platforms have no DPAPI and keep the plain value.

use base64::Engine as _;

/// Marks an encrypted value. A value without it is plain text, from an older
/// version or another platform, and is encrypted at the next save.
const PREFIX: &str = "dpapi:";

/// The stored form of `value`: `dpapi:<base64>` on Windows, else `value` as is.
pub fn seal(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    match protect(value.as_bytes()) {
        Ok(Some(sealed)) => format!(
            "{PREFIX}{}",
            base64::engine::general_purpose::STANDARD.encode(sealed)
        ),
        Ok(None) => value.to_string(),
        Err(e) => {
            log::warn!("{e:#}; the secret is saved without encryption");
            value.to_string()
        }
    }
}

/// The plain value of a stored secret. One that can't be decrypted (copied
/// from another user or PC) comes back empty, so it has to be entered again.
pub fn open(stored: &str) -> String {
    let Some(encoded) = stored.strip_prefix(PREFIX) else {
        return stored.to_string();
    };
    let plain = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(anyhow::Error::from)
        .and_then(|sealed| unprotect(&sealed))
        .and_then(|plain| Ok(String::from_utf8(plain)?));
    plain.unwrap_or_else(|e| {
        log::warn!("a saved secret can't be read on this account: {e:#}");
        String::new()
    })
}

/// For `#[serde(with = "crate::secure::string")]` on a secret `String` field.
pub mod string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &str, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&super::seal(value))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
        Ok(super::open(&String::deserialize(d)?))
    }
}

/// Encrypts `plain` for the current Windows user.
#[cfg(windows)]
fn protect(plain: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };
    let input = blob_of(plain);
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: `input` points into `plain`, which outlives the call. DPAPI only
    // reads it, and allocates `output` itself.
    let ok = unsafe {
        CryptProtectData(
            &input,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        anyhow::bail!("encrypting failed: {}", std::io::Error::last_os_error());
    }
    Ok(Some(take(output)))
}

/// Decrypts what `protect` returned for the same Windows user.
#[cfg(windows)]
fn unprotect(sealed: &[u8]) -> anyhow::Result<Vec<u8>> {
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };
    let input = blob_of(sealed);
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: as in `protect`.
    let ok = unsafe {
        CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        anyhow::bail!("decrypting failed: {}", std::io::Error::last_os_error());
    }
    Ok(take(output))
}

#[cfg(windows)]
fn blob_of(bytes: &[u8]) -> windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB {
    windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr().cast_mut(),
    }
}

/// Copies DPAPI's output and frees it.
#[cfg(windows)]
fn take(blob: windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB) -> Vec<u8> {
    if blob.pbData.is_null() {
        return Vec::new();
    }
    // SAFETY: DPAPI returned `cbData` bytes at `pbData`, allocated with
    // LocalAlloc, which LocalFree releases once they are copied.
    unsafe {
        let bytes = std::slice::from_raw_parts(blob.pbData, blob.cbData as usize).to_vec();
        windows_sys::Win32::Foundation::LocalFree(blob.pbData.cast());
        bytes
    }
}

#[cfg(not(windows))]
fn protect(_plain: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
    Ok(None)
}

#[cfg(not(windows))]
fn unprotect(_sealed: &[u8]) -> anyhow::Result<Vec<u8>> {
    anyhow::bail!("DPAPI is only available on Windows")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_values_from_older_versions_are_read_as_is() {
        assert_eq!(open("sk-plain-key"), "sk-plain-key");
        assert_eq!(open(""), "");
    }

    #[test]
    fn a_damaged_value_comes_back_empty() {
        assert_eq!(open("dpapi:not base64!"), "");
        assert_eq!(open("dpapi:AAAA"), "");
    }

    #[cfg(windows)]
    #[test]
    fn sealed_values_hide_the_secret_and_open_again() {
        let sealed = seal("sk-secret-123");
        assert!(sealed.starts_with(PREFIX));
        assert!(!sealed.contains("sk-secret-123"));
        assert_eq!(open(&sealed), "sk-secret-123");
        assert_eq!(seal(""), "");
    }
}
