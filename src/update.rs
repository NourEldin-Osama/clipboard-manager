//! Updates from GitHub Releases.
//!
//! Each release publishes `updater.json` next to the exe. Its `signed` field is
//! one line with every value this app uses: version, build number, size and
//! SHA-256 of the exe, and the download and notes links. `signed_signature` is
//! the Ed25519 signature of that line, made with a key that only the release
//! workflow has. The app reads the values only from the signed line, so a
//! changed file can't offer an older exe as a new one, and it installs a
//! download only if its size and SHA-256 match.
//!
//! Installing: Windows won't overwrite a running exe but will rename it, so the
//! running exe becomes `clipboard-manager.old`, the new one takes its name, and
//! the app restarts. The `.old` file is removed on the next start.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::models::Progress;

/// Verifies release signatures. Its private half is the `UPDATE_SIGNING_KEY` repo secret.
const PUBLIC_KEY: &str = "87vyKkh1uUjEosXCDL+KH0+9XjQ6xu+MrlCB9ETs9TM=";

const MANIFEST_URL: &str =
    "https://github.com/NourEldin-Osama/clipboard-manager/releases/latest/download/updater.json";

/// The first word of the signed line. A later format gets a new word, which
/// this version refuses instead of misreading.
const SIGNED_FORMAT: &str = "clipboard-manager-release-v1";

/// The exe is about 30 MB; a release that claims more than this is refused.
const MAX_DOWNLOAD_BYTES: u64 = 200 * 1024 * 1024;

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
        Some(b) => crate::i18n::t!("update-version-label", version = VERSION, build = b),
        None => crate::i18n::t!("update-development-build", version = VERSION),
    }
}

/// The part of `updater.json` this app reads. It is kept as it came, so a
/// staged update can be checked again before it's installed.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Manifest {
    signed: String,
    signed_signature: String,
}

/// A release, with values taken from a manifest whose signature checked out.
#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub build: u32,
    pub notes_url: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    manifest: Manifest,
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

impl Manifest {
    /// The release in the signed line, if `key` signed it and its values are
    /// ones the app accepts: a size within the limit, and https links.
    fn verify(self, key: &[u8; 32]) -> Result<Release> {
        check_signature(key, self.signed.as_bytes(), &self.signed_signature)
            .context("the update description isn't signed by this app's releases")?;
        let line = self.signed.clone();
        let mut words = line.split(' ');
        if words.next() != Some(SIGNED_FORMAT) {
            bail!("the update description has a format this version doesn't know");
        }
        let mut values = std::collections::HashMap::new();
        for word in words {
            let (name, value) = word
                .split_once('=')
                .context("the update description is damaged")?;
            if values.insert(name, value).is_some() {
                bail!("the update description repeats {name}");
            }
        }
        let value = |name: &str| {
            values
                .get(name)
                .copied()
                .with_context(|| format!("the update description lacks {name}"))
        };
        let release = Release {
            version: value("version")?.to_string(),
            build: value("build")?
                .parse()
                .context("the update's build number")?,
            notes_url: value("notes_url")?.to_string(),
            url: value("url")?.to_string(),
            size: value("size")?.parse().context("the update's size")?,
            sha256: value("sha256")?.to_string(),
            manifest: self,
        };
        if release.size == 0 || release.size > MAX_DOWNLOAD_BYTES {
            bail!(
                "the update claims {} bytes; at most {MAX_DOWNLOAD_BYTES} are allowed",
                release.size
            );
        }
        for link in [&release.url, &release.notes_url] {
            if !link.starts_with("https://") {
                bail!("the update's links must use https: {link}");
            }
        }
        Ok(release)
    }
}

/// `PUBLIC_KEY` as bytes.
fn public_key() -> Result<[u8; 32]> {
    base64::engine::general_purpose::STANDARD
        .decode(PUBLIC_KEY)?
        .try_into()
        .map_err(|_| anyhow!("bad public key"))
}

/// `signature` (base64) is `key`'s Ed25519 signature of `message`.
fn check_signature(key: &[u8; 32], message: &[u8], signature: &str) -> Result<()> {
    let signature: [u8; 64] = base64::engine::general_purpose::STANDARD
        .decode(signature.trim())?
        .try_into()
        .map_err(|_| anyhow!("bad signature"))?;
    ed25519_dalek::VerifyingKey::from_bytes(key)?
        .verify_strict(message, &ed25519_dalek::Signature::from_bytes(&signature))
        .map_err(|_| anyhow!("bad signature"))
}

/// The latest release's manifest. `CLIPBOARD_MANAGER_UPDATE_URL` points
/// elsewhere for testing; the signature check still applies, and only a
/// test server on this PC may use plain http.
pub fn check() -> Result<Release> {
    let custom = std::env::var("CLIPBOARD_MANAGER_UPDATE_URL").ok();
    let url = custom.as_deref().unwrap_or(MANIFEST_URL);
    let https_only = custom.is_none() || !crate::net::is_on_this_pc(url);
    let mut resp = crate::net::quick()
        .get(url)
        .config()
        .https_only(https_only)
        .build()
        .call()
        .with_context(|| format!("checking {url}"))?;
    let manifest: Manifest = resp
        .body_mut()
        .read_json()
        .context("reading the update manifest")?;
    manifest.verify(&public_key()?)
}

/// Where the next exe is put before it's swapped in: next to `exe`, the
/// running one.
fn staged_path(exe: &Path) -> PathBuf {
    exe.with_extension("new")
}

/// The manifest of the staged exe, so it can be checked again before a
/// later start installs it.
fn staged_manifest_path(exe: &Path) -> PathBuf {
    exe.with_extension("new.json")
}

/// Downloads and verifies the release. Returns the verified exe, ready to install.
pub fn download(r: &Release, p: &Arc<Progress>) -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let staged = staged_path(&exe);
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
        .config()
        .https_only(true)
        .build()
        .call()
        .with_context(|| format!("downloading {}", r.url))?;
    p.total.store(r.size, Ordering::Relaxed);
    // More bytes than the signed size end the download with an error.
    let mut reader = resp.body_mut().with_config().limit(r.size).reader();
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
    std::fs::write(staged_manifest_path(&exe), serde_json::to_vec(&r.manifest)?)?;
    Ok(staged)
}

/// An update downloaded earlier but never installed (a Windows shutdown ends
/// the app without the install on quit): checked again against its signed
/// manifest, then put in place. Returns the exe to start, or `None` when
/// there is nothing newer to install.
pub fn install_staged() -> Result<Option<PathBuf>> {
    let exe = std::env::current_exe()?;
    let installed = install_staged_at(&exe, &public_key()?, Release::is_newer)?;
    Ok(installed.then_some(exe))
}

/// `install_staged` for the exe at `exe`: the update must be signed by `key`,
/// and goes in only when `is_newer` says so. Returns whether it went in.
fn install_staged_at(
    exe: &Path,
    key: &[u8; 32],
    is_newer: impl Fn(&Release) -> bool,
) -> Result<bool> {
    let staged = staged_path(exe);
    if !staged.is_file() {
        return Ok(false);
    }
    let manifest =
        std::fs::read(staged_manifest_path(exe)).context("reading the staged update's manifest")?;
    let manifest: Manifest = serde_json::from_slice(&manifest)?;
    let release = manifest.verify(key)?;
    if !is_newer(&release) {
        return Ok(false);
    }
    verify(&std::fs::read(&staged)?, &release)?;
    install_at(exe, &staged)?;
    Ok(true)
}

/// The download is the exe the release workflow signed: its size and SHA-256
/// match the signed line.
fn verify(bytes: &[u8], r: &Release) -> Result<()> {
    use sha2::{Digest, Sha256};
    if bytes.len() as u64 != r.size {
        bail!(
            "the download is {} bytes, not the {} the release lists",
            bytes.len(),
            r.size
        );
    }
    let sum: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if !sum.eq_ignore_ascii_case(r.sha256.trim()) {
        bail!("the download is damaged (checksum mismatch)");
    }
    Ok(())
}

/// Puts the verified exe in place of the running one. Returns the exe path.
pub fn install(staged: &Path) -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    install_at(&exe, staged)?;
    Ok(exe)
}

/// Puts `staged` in place of `exe`, which is kept as `.old`.
fn install_at(exe: &Path, staged: &Path) -> Result<()> {
    let old = exe.with_extension("old");
    // Same permissions as the running exe (on Linux and macOS: executable).
    std::fs::set_permissions(staged, std::fs::metadata(exe)?.permissions())?;
    let _ = std::fs::remove_file(&old);
    std::fs::rename(exe, &old).with_context(|| format!("moving {} aside", exe.display()))?;
    if let Err(e) = std::fs::rename(staged, exe) {
        // Put the running version back rather than leave no exe at all.
        if let Err(back) = std::fs::rename(&old, exe) {
            return Err(e).context(format!(
                "putting the new version in place (putting {} back failed too: {back})",
                exe.display()
            ));
        }
        return Err(e).context("putting the new version in place");
    }
    Ok(())
}

/// Undoes `install`: the previous exe goes back in place.
pub fn roll_back() -> Result<()> {
    roll_back_at(&std::env::current_exe()?)
}

fn roll_back_at(exe: &Path) -> Result<()> {
    std::fs::rename(exe.with_extension("old"), exe).context("restoring the previous version")?;
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
        clean_up_at(&exe);
    }
}

fn clean_up_at(exe: &Path) {
    for ext in ["old", "new", "new.json", "part"] {
        let _ = std::fs::remove_file(exe.with_extension(ext));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer as _;

    /// Made with the real release key:
    /// `printf 'hello update\n' | openssl pkeyutl -sign -rawin`.
    const RELEASE_KEY_SIGNATURE: &str =
        "vh/Bl0j3js13ICyaRM0Nrhpn9KZJiFwXtNwYOonsEtsN4BpFSgeUFyeIDnXZV8PpdH4BZSWnrHSaGsbzX6lKAA==";

    #[test]
    fn the_built_in_key_is_the_release_key() {
        let key = public_key().unwrap();
        check_signature(&key, b"hello update\n", RELEASE_KEY_SIGNATURE).unwrap();
        assert!(check_signature(&key, b"hello update!\n", RELEASE_KEY_SIGNATURE).is_err());
    }

    /// A key for tests only; the real one never leaves the release workflow.
    fn test_key() -> ed25519_dalek::SigningKey {
        ed25519_dalek::SigningKey::from_bytes(&[7; 32])
    }

    fn test_public_key() -> [u8; 32] {
        test_key().verifying_key().to_bytes()
    }

    fn signed(line: &str) -> Manifest {
        Manifest {
            signed: line.to_string(),
            signed_signature: base64::engine::general_purpose::STANDARD
                .encode(test_key().sign(line.as_bytes()).to_bytes()),
        }
    }

    /// The line the release workflow signs, filled with test values. Its
    /// format comes from the workflow file, so the two can't drift apart.
    fn workflow_line(version: &str, build: u32, size: u64, sha256: &str) -> String {
        let workflow = include_str!("../.github/workflows/build-windows.yml");
        let start = workflow
            .find(&format!("printf '{SIGNED_FORMAT} "))
            .expect("the workflow signs a release line")
            + "printf '".len();
        let format = &workflow[start..start + workflow[start..].find('\'').unwrap()];
        let values = [
            version.to_string(),
            build.to_string(),
            size.to_string(),
            sha256.to_string(),
            "https://example.com/download/app.exe".to_string(),
            "https://example.com/tag/v1".to_string(),
        ];
        let mut line = format.to_string();
        for value in values {
            line = line.replacen("%s", &value, 1);
        }
        assert!(
            !line.contains('%'),
            "the workflow line has values the test doesn't fill"
        );
        line
    }

    #[test]
    fn the_signed_line_gives_the_release() {
        let sha = "ab".repeat(32);
        let line = workflow_line("0.3.0", 42, 1234, &sha);
        let r = signed(&line).verify(&test_public_key()).unwrap();
        assert_eq!((r.version.as_str(), r.build, r.size), ("0.3.0", 42, 1234));
        assert_eq!(r.sha256, sha);
        assert_eq!(r.url, "https://example.com/download/app.exe");
        assert_eq!(r.notes_url, "https://example.com/tag/v1");
    }

    #[test]
    fn a_changed_line_or_another_key_is_refused() {
        let line = workflow_line("0.3.0", 42, 1234, &"ab".repeat(32));
        let mut changed = signed(&line);
        changed.signed = changed.signed.replace("build=42", "build=99");
        assert!(changed.verify(&test_public_key()).is_err());
        let release_key = public_key().unwrap();
        assert!(signed(&line).verify(&release_key).is_err(), "another key");
    }

    #[test]
    fn unsafe_values_are_refused() {
        let sha = "ab".repeat(32);
        let line = workflow_line("0.3.0", 42, 1234, &sha);
        for (what, bad) in [
            ("no size", line.replace("size=1234", "size=0")),
            (
                "too big",
                line.replace("size=1234", &format!("size={}", MAX_DOWNLOAD_BYTES + 1)),
            ),
            (
                "http download",
                line.replacen("url=https://", "url=http://", 1),
            ),
            (
                "http notes",
                line.replace("notes_url=https://", "notes_url=http://"),
            ),
            (
                "unknown format",
                line.replacen(SIGNED_FORMAT, "clipboard-manager-release-v9", 1),
            ),
            ("missing value", line.replace(&format!(" sha256={sha}"), "")),
            ("repeated value", format!("{line} build=43")),
        ] {
            assert!(signed(&bad).verify(&test_public_key()).is_err(), "{what}");
        }
    }

    #[test]
    fn a_download_must_match_the_signed_size_and_checksum() {
        use sha2::{Digest, Sha256};
        let bytes = b"new exe".to_vec();
        let sha: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let line = workflow_line("0.3.0", 42, bytes.len() as u64, &sha);
        let r = signed(&line).verify(&test_public_key()).unwrap();
        verify(&bytes, &r).unwrap();
        assert!(verify(b"new exe!", &r).is_err(), "longer");
        let mut changed = bytes.clone();
        changed[0] ^= 1;
        assert!(verify(&changed, &r).is_err(), "same size, other bytes");
    }

    #[test]
    fn a_staged_manifest_is_checked_again() {
        let line = workflow_line("0.3.0", 42, 1234, &"ab".repeat(32));
        let r = signed(&line).verify(&test_public_key()).unwrap();
        let saved: Manifest =
            serde_json::from_slice(&serde_json::to_vec(&r.manifest).unwrap()).unwrap();
        assert_eq!(saved.verify(&test_public_key()).unwrap().build, 42);
    }

    #[test]
    fn compares_versions() {
        assert!(numbers("0.10.0") > numbers("0.9.9"));
        assert!((numbers("0.1.0"), 13) > (numbers("0.1.0"), 12));
        assert!((numbers("0.2.0"), 1) > (numbers("0.1.0"), 99));
    }

    /// A running exe in a folder of its own; the test removes the folder.
    fn running_exe(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cbm-self-update-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("clipboard-manager.exe");
        std::fs::write(&exe, "running version").unwrap();
        exe
    }

    fn read(path: &Path) -> String {
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn installing_keeps_the_running_version_for_a_roll_back() {
        let exe = running_exe("install");
        let staged = staged_path(&exe);
        std::fs::write(&staged, "new version").unwrap();
        install_at(&exe, &staged).unwrap();
        assert_eq!(read(&exe), "new version");
        assert_eq!(read(&exe.with_extension("old")), "running version");
        assert!(!staged.exists());
        roll_back_at(&exe).unwrap();
        assert_eq!(read(&exe), "running version");
        assert!(!exe.with_extension("old").exists());
        std::fs::remove_dir_all(exe.parent().unwrap()).unwrap();
    }

    /// Windows won't move a file another program holds open without sharing
    /// it, so the new exe can't take the running one's place.
    #[cfg(windows)]
    #[test]
    fn a_failed_install_puts_the_running_version_back() {
        use std::os::windows::fs::OpenOptionsExt;
        let exe = running_exe("failed");
        let staged = staged_path(&exe);
        std::fs::write(&staged, "new version").unwrap();
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&staged)
            .unwrap();
        let error = install_at(&exe, &staged).unwrap_err();
        drop(held);
        assert!(
            format!("{error:#}").contains("putting the new version in place"),
            "{error:#}"
        );
        assert_eq!(read(&exe), "running version");
        assert!(!exe.with_extension("old").exists());
        std::fs::remove_dir_all(exe.parent().unwrap()).unwrap();
    }

    #[test]
    fn cleaning_up_removes_only_what_an_update_leaves() {
        let exe = running_exe("clean");
        let left = ["old", "new", "new.json", "part"].map(|ext| exe.with_extension(ext));
        for file in &left {
            std::fs::write(file, "left over").unwrap();
        }
        let other = exe.with_file_name("notes.txt");
        std::fs::write(&other, "kept").unwrap();
        clean_up_at(&exe);
        for file in &left {
            assert!(!file.exists(), "{}", file.display());
        }
        assert!(exe.exists() && other.exists());
        std::fs::remove_dir_all(exe.parent().unwrap()).unwrap();
    }

    /// Puts `bytes` next to `exe` with a manifest signed by the test key, as
    /// `download` does.
    fn stage(exe: &Path, bytes: &str) {
        use sha2::{Digest, Sha256};
        let sha: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let line = workflow_line("9.0.0", 42, bytes.len() as u64, &sha);
        std::fs::write(staged_path(exe), bytes).unwrap();
        let manifest = serde_json::to_vec(&signed(&line)).unwrap();
        std::fs::write(staged_manifest_path(exe), manifest).unwrap();
    }

    #[test]
    fn a_staged_update_goes_in_only_when_newer_and_unchanged() {
        let exe = running_exe("staged");
        let key = test_public_key();
        assert!(
            !install_staged_at(&exe, &key, |_| true).unwrap(),
            "nothing staged"
        );
        stage(&exe, "new version");
        assert!(
            !install_staged_at(&exe, &key, |_| false).unwrap(),
            "not newer"
        );
        let release_key = public_key().unwrap();
        assert!(
            install_staged_at(&exe, &release_key, |_| true).is_err(),
            "another key"
        );
        // Changed on disk after the download: same size, other bytes.
        std::fs::write(staged_path(&exe), "new versioN").unwrap();
        assert!(install_staged_at(&exe, &key, |_| true).is_err(), "changed");
        assert_eq!(read(&exe), "running version");
        stage(&exe, "new version");
        assert!(install_staged_at(&exe, &key, |_| true).unwrap());
        assert_eq!(read(&exe), "new version");
        assert_eq!(read(&exe.with_extension("old")), "running version");
        std::fs::remove_dir_all(exe.parent().unwrap()).unwrap();
    }
}
