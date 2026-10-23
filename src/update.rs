//! Updates from GitHub Releases.
//!
//! Each release publishes `updater.json` next to the exe: version, build
//! number, download URL, SHA-256 and an Ed25519 signature of the exe made with
//! a key that only the release workflow has. A download is installed only if
//! both match, checked against `PUBLIC_KEY` below.
//!
//! Installing: Windows won't overwrite a running exe but will rename it, so the
//! running exe becomes `clipboard-manager.old`, the new one takes its name, and
//! the app restarts. The `.old` file is removed on the next start.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::models::Progress;

/// Verifies release signatures. Its private half is the `UPDATE_SIGNING_KEY` repo secret.
const PUBLIC_KEY: &str = "87vyKkh1uUjEosXCDL+KH0+9XjQ6xu+MrlCB9ETs9TM=";

const MANIFEST_URL: &str =
    "https://github.com/NourEldin-Osama/clipboard-manager/releases/latest/download/updater.json";

/// Passed to the new exe after an update, so it waits for this one to exit.
pub const AFTER_UPDATE_ARG: &str = "--after-update";

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The release workflow's run number, set at build time. Local builds have
/// none, and don't update themselves.
pub fn build() -> Option<u32> {
    option_env!("BUILD_NUMBER").and_then(|b| b.parse().ok())
}

/// "0.1.0 (build 12)", or "0.1.0 (development build)".
pub fn current_label() -> String {
    match build() {
        Some(b) => format!("{VERSION} (build {b})"),
        None => format!("{VERSION} ({})", crate::i18n::tr("development build")),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Release {
    pub version: String,
    pub build: u32,
    pub notes_url: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    pub signature: String,
}

impl Release {
    /// "1.4.0 (build 52)", in the interface language.
    pub fn label(&self) -> String {
        crate::i18n::t!(
            "update-version-label",
            version = self.version.as_str(),
            build = self.build
        )
    }

    /// Remembered by "Skip this version".
    pub fn key(&self) -> String {
        format!("{}+{}", self.version, self.build)
    }

    /// Newer than the running build.
    pub fn is_newer(&self) -> bool {
        build().is_some_and(|b| (numbers(&self.version), self.build) > (numbers(VERSION), b))
    }
}

fn numbers(v: &str) -> Vec<u64> {
    v.split(['.', '-', '+'])
        .map_while(|p| p.parse().ok())
        .collect()
}

/// The latest release's manifest. `CLIPBOARD_MANAGER_UPDATE_URL` points
/// elsewhere for testing; the signature check still applies.
pub fn check() -> Result<Release> {
    let url =
        std::env::var("CLIPBOARD_MANAGER_UPDATE_URL").unwrap_or_else(|_| MANIFEST_URL.to_string());
    let mut resp = crate::net::quick()
        .get(&url)
        .call()
        .with_context(|| format!("checking {url}"))?;
    let release: Release = resp
        .body_mut()
        .read_json()
        .context("reading the update manifest")?;
    Ok(release)
}

/// Where the next exe is put before it's swapped in: next to the running one.
fn staged_path() -> Result<PathBuf> {
    Ok(std::env::current_exe()?.with_extension("new"))
}

/// The manifest of the staged exe, so it can be checked again before a
/// later start installs it.
fn staged_manifest_path() -> Result<PathBuf> {
    Ok(std::env::current_exe()?.with_extension("new.json"))
}

/// Downloads and verifies the release. Returns the verified exe, ready to install.
pub fn download(r: &Release, p: &Arc<Progress>) -> Result<PathBuf> {
    let staged = staged_path()?;
    let part = staged.with_extension("part");
    // A folder the app can't write to (Program Files, say) fails here, before
    // the download, instead of after it, again every day.
    let mut out = std::fs::File::create(&part).with_context(|| {
        format!(
            "the app's folder can't be written ({}); download the new version by hand",
            part.display()
        )
    })?;
    // The exe is about 30 MB: half an hour covers a very slow line.
    let mut resp = crate::net::download(Duration::from_secs(30 * 60))
        .get(&r.url)
        .call()
        .with_context(|| format!("downloading {}", r.url))?;
    p.total.store(r.size, Ordering::Relaxed);
    let mut reader = resp.body_mut().with_config().limit(u64::MAX).reader();
    let mut bytes = Vec::with_capacity(r.size as usize);
    let mut buf = vec![0u8; 1 << 16];
    loop {
        if p.cancel.load(Ordering::Relaxed) {
            bail!("cancelled");
        }
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&buf[..n]);
        p.done.store(bytes.len() as u64, Ordering::Relaxed);
    }
    verify(&bytes, r)?;
    out.write_all(&bytes)
        .with_context(|| format!("writing {}", part.display()))?;
    out.sync_all()?;
    drop(out);
    std::fs::rename(&part, &staged)?;
    std::fs::write(staged_manifest_path()?, serde_json::to_vec(r)?)?;
    Ok(staged)
}

/// An update downloaded earlier but never installed (a Windows shutdown ends
/// the app without the install on quit): checked again against its signed
/// manifest, then put in place. Returns the exe to start, or `None` when
/// there is nothing newer to install.
pub fn install_staged() -> Result<Option<PathBuf>> {
    let staged = staged_path()?;
    if !staged.is_file() {
        return Ok(None);
    }
    let manifest =
        std::fs::read(staged_manifest_path()?).context("reading the staged update's manifest")?;
    let release: Release = serde_json::from_slice(&manifest)?;
    if !release.is_newer() {
        return Ok(None);
    }
    verify(&std::fs::read(&staged)?, &release)?;
    install(&staged).map(Some)
}

/// The download is the one the release workflow signed, byte for byte.
fn verify(bytes: &[u8], r: &Release) -> Result<()> {
    use sha2::{Digest, Sha256};
    let sum: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if !sum.eq_ignore_ascii_case(r.sha256.trim()) {
        bail!("the download is damaged (checksum mismatch)");
    }
    let b64 = base64::engine::general_purpose::STANDARD;
    let key: [u8; 32] = b64
        .decode(PUBLIC_KEY)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("bad public key"))?;
    let sig: [u8; 64] = b64
        .decode(r.signature.trim())?
        .try_into()
        .map_err(|_| anyhow::anyhow!("bad signature"))?;
    let key = ed25519_dalek::VerifyingKey::from_bytes(&key)?;
    key.verify_strict(bytes, &ed25519_dalek::Signature::from_bytes(&sig))
        .map_err(|_| anyhow::anyhow!("the download isn't signed by this app's releases"))?;
    Ok(())
}

/// Puts the verified exe in place of the running one. Returns the exe path.
pub fn install(staged: &Path) -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let old = exe.with_extension("old");
    // Same permissions as the running exe (on Linux and macOS: executable).
    std::fs::set_permissions(staged, std::fs::metadata(&exe)?.permissions())?;
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).with_context(|| format!("moving {} aside", exe.display()))?;
    if let Err(e) = std::fs::rename(staged, &exe) {
        // Put the running version back rather than leave no exe at all.
        if let Err(back) = std::fs::rename(&old, &exe) {
            return Err(e).context(format!(
                "putting the new version in place (putting {} back failed too: {back})",
                exe.display()
            ));
        }
        return Err(e).context("putting the new version in place");
    }
    Ok(exe)
}

/// Undoes `install`: the previous exe goes back in place.
pub fn roll_back() -> Result<()> {
    let exe = std::env::current_exe()?;
    std::fs::rename(exe.with_extension("old"), &exe).context("restoring the previous version")?;
    Ok(())
}

/// Starts the updated app; the caller then quits.
pub fn relaunch(exe: &Path) -> Result<()> {
    std::process::Command::new(exe)
        .arg(AFTER_UPDATE_ARG)
        .spawn()
        .context("starting the new version")?;
    Ok(())
}

/// Removes what a finished or abandoned update leaves behind.
pub fn clean_up() {
    if let Ok(exe) = std::env::current_exe() {
        for ext in ["old", "new", "new.json", "part"] {
            let _ = std::fs::remove_file(exe.with_extension(ext));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Signed with the real release key: `printf 'hello update\n' | openssl pkeyutl -sign -rawin`.
    fn fixture() -> (Vec<u8>, Release) {
        let r = Release {
            version: "0.1.0".into(),
            build: 1,
            notes_url: String::new(),
            url: String::new(),
            size: 13,
            sha256: "85ebff07175145475a29610a59842c3f0e9b74d72ce23b3c535bd01eeca62027".into(),
            signature: "vh/Bl0j3js13ICyaRM0Nrhpn9KZJiFwXtNwYOonsEtsN4BpFSgeUFyeIDnXZV8PpdH4BZSWnrHSaGsbzX6lKAA==".into(),
        };
        (b"hello update\n".to_vec(), r)
    }

    #[test]
    fn accepts_a_signed_release() {
        let (bytes, r) = fixture();
        verify(&bytes, &r).unwrap();
    }

    #[test]
    fn rejects_tampering() {
        let (mut bytes, mut r) = fixture();
        bytes[0] ^= 1;
        assert!(verify(&bytes, &r).is_err(), "changed bytes");
        // Even with a matching checksum, the signature must hold.
        use sha2::{Digest, Sha256};
        r.sha256 = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert!(verify(&bytes, &r).is_err(), "re-hashed but unsigned");
    }

    #[test]
    fn compares_versions() {
        assert!(numbers("0.10.0") > numbers("0.9.9"));
        assert!((numbers("0.1.0"), 13) > (numbers("0.1.0"), 12));
        assert!((numbers("0.2.0"), 1) > (numbers("0.1.0"), 99));
    }

    #[test]
    fn a_saved_manifest_still_verifies_its_download() {
        let (bytes, r) = fixture();
        let saved: Release = serde_json::from_slice(&serde_json::to_vec(&r).unwrap()).unwrap();
        verify(&bytes, &saved).unwrap();
        let mut tampered = bytes.clone();
        tampered[0] ^= 1;
        assert!(verify(&tampered, &saved).is_err());
    }
}
