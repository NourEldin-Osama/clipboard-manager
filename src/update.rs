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
//! the app restarts. The new version keeps `.old` until it counts as working:
//! its window has drawn and stayed open for `WORKING_AFTER`. Until then a
//! marker next to the exe (`.update-pending`) counts its starts. When two of
//! them never got there, the third puts `.old` back, skips the failed version
//! in the settings and starts the previous version.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::download::Progress;

/// Verifies release signatures. Its private half is the `UPDATE_SIGNING_KEY` repo secret.
const PUBLIC_KEY: &str = "87vyKkh1uUjEosXCDL+KH0+9XjQ6xu+MrlCB9ETs9TM=";

const MANIFEST_URL: &str =
    "https://github.com/NourEldin-Osama/clipboard-manager/releases/latest/download/updater.json";

/// The first word of the signed line. A later format gets a new word, which
/// this version refuses instead of misreading.
const SIGNED_FORMAT: &str = "clipboard-manager-release-v1";

/// The exe is about 30 MB; a release that claims more than this is refused.
const MAX_DOWNLOAD_BYTES: u64 = 200 * 1024 * 1024;

/// Passed to the exe started after an update, or after putting the previous
/// version back, so it waits for this one to exit.
pub const AFTER_UPDATE_ARG: &str = "--after-update";

/// A new version counts as working once its window has drawn and stayed open
/// this long. Then the previous version is removed.
pub const WORKING_AFTER: Duration = Duration::from_secs(10);

/// The starts a new version gets to count as working. The start after them
/// puts the previous version back.
const STARTS_TO_WORK: u32 = 2;

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
        key_of(&self.version, self.build)
    }

    /// Newer than the running build.
    pub fn is_newer(&self) -> bool {
        build().is_some_and(|b| (numbers(&self.version), self.build) > (numbers(VERSION), b))
    }

    /// Newer than `other`: a later version, or a later build of the same
    /// version.
    pub fn is_newer_than(&self, other: &Release) -> bool {
        (numbers(&self.version), self.build) > (numbers(&other.version), other.build)
    }

    /// A release with made-up values and no signature, for tests of the
    /// window.
    #[cfg(test)]
    pub fn made_up(version: &str, build: u32, size: u64) -> Release {
        Release {
            version: version.to_string(),
            build,
            notes_url: "https://example.com/tag/v1".to_string(),
            url: "https://example.com/download/app.exe".to_string(),
            size,
            sha256: "ab".repeat(32),
            manifest: Manifest {
                signed: String::new(),
                signed_signature: String::new(),
            },
        }
    }
}

/// "1.4.0+52": how "Skip this version" and the update marker name a release.
fn key_of(version: &str, build: u32) -> String {
    format!("{version}+{build}")
}

/// The running build's key. Local builds have none.
fn running_key() -> Option<String> {
    build().map(|build| key_of(VERSION, build))
}

/// Whether the running version is newer than `earlier`, such as "0.5.1".
pub fn newer_than(earlier: &str) -> bool {
    numbers(VERSION) > numbers(earlier)
}

/// The notes for `version` from the change log built into the app, one per
/// entry, as plain text.
pub fn notes(version: &str) -> Vec<String> {
    notes_in(include_str!("../CHANGELOG.md"), version)
}

/// The entries under `## <version>`: each starts with "- ", and its lines
/// that start with two spaces go on. Bold marks and backticks are left out.
fn notes_in(changelog: &str, version: &str) -> Vec<String> {
    let heading = format!("## {version}");
    let mut notes: Vec<String> = Vec::new();
    let mut inside = false;
    for line in changelog.lines() {
        if line.starts_with("## ") {
            inside = line.trim_end() == heading;
        } else if !inside {
            continue;
        } else if let Some(first) = line.strip_prefix("- ") {
            notes.push(first.trim().to_string());
        } else if let (true, Some(note)) = (line.starts_with("  "), notes.last_mut()) {
            note.push(' ');
            note.push_str(line.trim());
        }
    }
    notes
        .into_iter()
        .map(|note| note.replace("**", "").replace('`', ""))
        .collect()
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

/// Downloads and verifies the release. Returns the verified exe, ready to
/// install. A download that stops is continued by the next one.
pub fn download(r: &Release, p: &Arc<Progress>) -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let staged = staged_path(&exe);
    // A folder the app can't write to (Program Files, say) fails here, before
    // the download, instead of after it, again every day.
    let part = staged.with_extension("part");
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&part)
        .with_context(|| {
            format!(
                "the app's folder can't be written ({}); download the new version by hand",
                part.display()
            )
        })?;
    crate::download::Download {
        url: &r.url,
        dest: &staged,
        size: Some(r.size),
        https_only: true,
        // The exe is about 30 MB: half an hour a try covers a very slow line.
        max_body: Duration::from_secs(30 * 60),
    }
    .run(p)?;
    if let Err(e) = verify(&std::fs::read(&staged)?, r) {
        // Not the signed file: no use, also not to continue.
        let _ = std::fs::remove_file(&staged);
        return Err(e);
    }
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
    install_at(exe, &staged, &release.key())?;
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

/// Puts the verified exe of `release` in place of the running one. Returns
/// the exe path.
pub fn install(release: &Release, staged: &Path) -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    install_at(&exe, staged, &release.key())?;
    Ok(exe)
}

/// Puts `staged`, the release with `key`, in place of `exe`, which is kept
/// as `.old` until the new version counts as working.
fn install_at(exe: &Path, staged: &Path, key: &str) -> Result<()> {
    let old = exe.with_extension("old");
    // Same permissions as the running exe (on Linux and macOS: executable).
    std::fs::set_permissions(staged, std::fs::metadata(exe)?.permissions())?;
    // Written first, so the new version never starts without it.
    let marker = Pending {
        key: key.to_string(),
        starts: 0,
    };
    write_pending(exe, &marker).context("marking the new version as not yet working")?;
    let _ = std::fs::remove_file(&old);
    let swapped = swap_in(exe, staged, &old, "putting the new version in place");
    if swapped.is_err() {
        remove_logged(&pending_path(exe));
    }
    swapped
}

/// Puts `replacement` in place of `exe`, which moves to `aside`. When the
/// second move fails, `exe` goes back rather than leave no exe at all.
/// `step` names the swap in the error.
fn swap_in(exe: &Path, replacement: &Path, aside: &Path, step: &str) -> Result<()> {
    std::fs::rename(exe, aside).with_context(|| format!("moving {} aside", exe.display()))?;
    if let Err(e) = std::fs::rename(replacement, exe) {
        if let Err(back) = std::fs::rename(aside, exe) {
            return Err(e).context(format!(
                "{step} (putting {} back failed too: {back})",
                exe.display()
            ));
        }
        return Err(e).context(step.to_string());
    }
    Ok(())
}

/// Undoes `install`: the previous exe goes back in place.
pub fn roll_back() -> Result<()> {
    roll_back_at(&std::env::current_exe()?)
}

fn roll_back_at(exe: &Path) -> Result<()> {
    std::fs::rename(exe.with_extension("old"), exe).context("restoring the previous version")?;
    remove_logged(&pending_path(exe));
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

/// Removes what a finished or abandoned update leaves behind. A partial
/// download stays, for the next download to continue, and so does the
/// previous version while the marker says the running one hasn't worked yet.
pub fn clean_up() {
    if let Ok(exe) = std::env::current_exe() {
        clean_up_at(&exe);
    }
}

fn clean_up_at(exe: &Path) {
    let mut left = vec![
        staged_path(exe),
        staged_manifest_path(exe),
        failed_path(exe),
    ];
    if !pending_path(exe).exists() {
        left.push(exe.with_extension("old"));
    }
    for file in left {
        let _ = std::fs::remove_file(file);
    }
}

/// Next to the exe from an install until the new version counts as working.
fn pending_path(exe: &Path) -> PathBuf {
    exe.with_extension("update-pending")
}

/// Where a new version that never worked goes when the previous one comes
/// back. The next start removes it.
fn failed_path(exe: &Path) -> PathBuf {
    exe.with_extension("failed")
}

/// What the update marker keeps.
#[derive(Debug, Deserialize, Serialize)]
struct Pending {
    /// The installed version's `Release::key`.
    key: String,
    /// Its starts so far. None of them counted as working, because that
    /// removes the marker.
    starts: u32,
}

/// The update marker next to `exe`, or `None` when there is none.
fn read_pending(exe: &Path) -> Result<Option<Pending>> {
    let bytes = match std::fs::read(pending_path(exe)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).context("reading the update marker"),
    };
    let pending = serde_json::from_slice(&bytes).context("reading the update marker")?;
    Ok(Some(pending))
}

fn write_pending(exe: &Path, pending: &Pending) -> Result<()> {
    std::fs::write(pending_path(exe), serde_json::to_vec(pending)?)?;
    Ok(())
}

/// What a start found about the last update.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Start {
    /// The running version needs no watching.
    Normal,
    /// The running version was just installed. The previous one stays until
    /// it counts as working (`mark_working`).
    Pending,
    /// The running version never worked: the previous one is back in its
    /// place and started, and this start ends.
    RolledBack,
}

/// Checked at every start, before anything else that could fail. Counts the
/// starts of a version just installed. When two of them never counted as
/// working, puts the previous version back, skips the failed one in the
/// settings at `config` (so the previous version doesn't install it again
/// by itself) and starts the previous version. A step that fails is logged,
/// and the running version goes on.
pub fn check_pending(config: &Path) -> Start {
    let after_update = std::env::args().any(|argument| argument == AFTER_UPDATE_ARG);
    match std::env::current_exe() {
        Ok(exe) => check_pending_at(
            &exe,
            running_key().as_deref(),
            after_update,
            config,
            start_previous,
        ),
        Err(e) => {
            log::warn!("finding the running exe: {e}");
            Start::Normal
        }
    }
}

/// `check_pending` for `exe`, the version named `key` (none for a local
/// build), started `after_update` or not. `start` starts the previous
/// version.
fn check_pending_at(
    exe: &Path,
    key: Option<&str>,
    after_update: bool,
    config: &Path,
    start: impl FnOnce(&Path) -> Result<()>,
) -> Start {
    let pending = match read_pending(exe) {
        Ok(Some(pending)) => pending,
        Ok(None) => return first_start_without_marker(exe, key, after_update),
        Err(e) => {
            log::warn!("{e:#}");
            remove_logged(&pending_path(exe));
            return Start::Normal;
        }
    };
    if key != Some(pending.key.as_str()) {
        // Another version runs: put back, or installed by hand.
        remove_logged(&pending_path(exe));
        return Start::Normal;
    }
    if pending.starts >= STARTS_TO_WORK {
        log::warn!(
            "version {} never worked in {} starts: putting the previous version back",
            pending.key,
            pending.starts
        );
        return match put_previous_back(exe, &pending.key, config, start) {
            Ok(()) => Start::RolledBack,
            Err(e) => {
                log::error!("putting the previous version back: {e:#}");
                Start::Pending
            }
        };
    }
    let counted = Pending {
        starts: pending.starts + 1,
        ..pending
    };
    log::info!(
        "version {} was just installed; this is its start {}",
        counted.key,
        counted.starts
    );
    if let Err(e) = write_pending(exe, &counted) {
        log::warn!("counting a start of the new version: {e:#}");
    }
    Start::Pending
}

/// A start with no marker. A version before 0.12.0 installs an update
/// without one, so the version it installed, started after the update with
/// the previous one still there, writes its own and counts this start. That
/// one can't catch a crash before this point, as the installer's marker can.
fn first_start_without_marker(exe: &Path, key: Option<&str>, after_update: bool) -> Start {
    let previous_there = exe.with_extension("old").is_file();
    let (Some(key), true, true) = (key, after_update, previous_there) else {
        return Start::Normal;
    };
    let first = Pending {
        key: key.to_string(),
        starts: 1,
    };
    match write_pending(exe, &first) {
        Ok(()) => {
            log::info!("version {key} was just installed by an older version; this is its start 1");
            Start::Pending
        }
        Err(e) => {
            log::warn!("marking the new version as not yet working: {e:#}");
            Start::Normal
        }
    }
}

/// Puts `.old` back in place of `exe`, the version named `key`, which never
/// worked. That one waits as `.failed` for the next start to remove it.
/// Then skips `key` in the settings at `config`, removes the marker and
/// starts the previous version with `start`. Once the previous version is in
/// place, a step that fails is logged and the next steps still run.
fn put_previous_back(
    exe: &Path,
    key: &str,
    config: &Path,
    start: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    let old = exe.with_extension("old");
    if !old.is_file() {
        bail!("the previous version isn't there any more");
    }
    let failed = failed_path(exe);
    let _ = std::fs::remove_file(&failed);
    swap_in(exe, &old, &failed, "putting the previous version in place")?;
    if let Err(e) = skip_in_settings(config, key) {
        log::warn!("skipping version {key} in the settings: {e:#}");
    }
    remove_logged(&pending_path(exe));
    start(exe)
}

/// Saves `key` as the skipped update in the settings at `config`. Settings
/// that can't be read are left as they are.
fn skip_in_settings(config: &Path, key: &str) -> Result<()> {
    let (mut settings, problem) = crate::config::Config::load(config);
    if let Some(problem) = problem {
        bail!("the settings can't be read: {}", problem.error);
    }
    settings.skipped_update = key.to_string();
    settings.save(config)
}

/// Starts the exe at `exe` the way this copy started (hidden at sign-in,
/// say), once this copy has quit.
fn start_previous(exe: &Path) -> Result<()> {
    let arguments = std::env::args_os()
        .skip(1)
        .filter(|argument| *argument != *AFTER_UPDATE_ARG);
    std::process::Command::new(exe)
        .args(arguments)
        .arg(AFTER_UPDATE_ARG)
        .spawn()
        .context("starting the previous version")?;
    Ok(())
}

/// The running version counts as working: the marker and the previous
/// version go.
pub fn mark_working() {
    match std::env::current_exe() {
        Ok(exe) => mark_working_at(&exe),
        Err(e) => log::warn!("finding the running exe: {e}"),
    }
}

fn mark_working_at(exe: &Path) {
    log::info!("this version works: removing the previous one");
    remove_logged(&pending_path(exe));
    remove_logged(&exe.with_extension("old"));
}

/// Removes `file`. A failure, other than finding no file, is logged.
fn remove_logged(file: &Path) {
    match std::fs::remove_file(file) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => log::warn!("removing {}: {e}", crate::system::without_user(file)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer as _;

    #[test]
    fn notes_are_the_entries_of_one_version() {
        let changelog = "# Changelog\r\n\r\nWhat changed.\r\n\r\n## 0.6.0\r\n\r\n\
            - **Reset all settings** keeps your servers.\r\n\
            - Web addresses show `ar.wikipedia.org` as you read\r\n  them, \
            and search finds them.\r\n\r\n## 0.5.1\r\n\r\n- An older note.\r\n";
        assert_eq!(
            notes_in(changelog, "0.6.0"),
            [
                "Reset all settings keeps your servers.",
                "Web addresses show ar.wikipedia.org as you read them, and search finds them.",
            ]
        );
        assert_eq!(notes_in(changelog, "0.5.1"), ["An older note."]);
        assert!(notes_in(changelog, "0.4.0").is_empty());
        // The change log in the app has notes for the running version.
        assert!(!notes(VERSION).is_empty());
        // Each version's notes are whole sentences: a line wrapped without
        // its two spaces would cut a note short or split it in two.
        let built_in = include_str!("../CHANGELOG.md");
        for version in built_in.lines().filter_map(|line| line.strip_prefix("## ")) {
            for note in notes_in(built_in, version.trim_end()) {
                assert!(note.ends_with('.'), "{version}: {note}");
            }
        }
    }

    #[test]
    fn a_newer_version_compares_by_its_numbers() {
        assert!(newer_than("0.0.1"));
        assert!(!newer_than(VERSION));
        assert!(!newer_than("999.0.0"));
    }

    #[test]
    fn a_release_is_newer_by_its_version_then_its_build() {
        let sha = "ab".repeat(32);
        let release = |version: &str, build: u32| {
            signed(&workflow_line(version, build, 1234, &sha))
                .verify(&test_public_key())
                .unwrap()
        };
        let waiting = release("0.7.1", 28);
        assert!(release("0.7.2", 29).is_newer_than(&waiting));
        assert!(
            release("0.10.0", 3).is_newer_than(&waiting),
            "by number, not text"
        );
        assert!(
            release("0.7.1", 30).is_newer_than(&waiting),
            "a later build of the same version"
        );
        assert!(!release("0.7.1", 28).is_newer_than(&waiting));
        assert!(!release("0.7.0", 40).is_newer_than(&waiting));
    }

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
        install_at(&exe, &staged, &new_key()).unwrap();
        assert_eq!(read(&exe), "new version");
        assert_eq!(read(&exe.with_extension("old")), "running version");
        assert!(!staged.exists());
        roll_back_at(&exe).unwrap();
        assert_eq!(read(&exe), "running version");
        assert!(!exe.with_extension("old").exists());
        assert!(!pending_path(&exe).exists(), "the marker goes too");
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
        let error = install_at(&exe, &staged, &new_key()).unwrap_err();
        drop(held);
        assert!(
            format!("{error:#}").contains("putting the new version in place"),
            "{error:#}"
        );
        assert_eq!(read(&exe), "running version");
        assert!(!exe.with_extension("old").exists());
        assert!(!pending_path(&exe).exists());
        std::fs::remove_dir_all(exe.parent().unwrap()).unwrap();
    }

    #[test]
    fn cleaning_up_removes_what_an_update_leaves_but_a_partial_download() {
        let exe = running_exe("clean");
        let left = ["old", "new", "new.json", "failed"].map(|ext| exe.with_extension(ext));
        let partial = ["part", "part-tag"].map(|ext| exe.with_extension(ext));
        for file in left.iter().chain(&partial) {
            std::fs::write(file, "left over").unwrap();
        }
        let other = exe.with_file_name("notes.txt");
        std::fs::write(&other, "kept").unwrap();
        clean_up_at(&exe);
        for file in &left {
            assert!(!file.exists(), "{}", file.display());
        }
        for file in &partial {
            assert!(
                file.exists(),
                "kept for the next download: {}",
                file.display()
            );
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
        let marker = read_pending(&exe).unwrap().expect("a marker");
        assert_eq!((marker.key.as_str(), marker.starts), ("9.0.0+42", 0));
        std::fs::remove_dir_all(exe.parent().unwrap()).unwrap();
    }

    /// The key of the new version the tests install.
    fn new_key() -> String {
        Release::made_up("9.0.0", 42, 11).key()
    }

    /// The settings next to the test's exe.
    fn settings(exe: &Path) -> PathBuf {
        exe.with_file_name("config.json")
    }

    /// The running exe, with the new version installed in its place.
    fn installed(name: &str) -> PathBuf {
        let exe = running_exe(name);
        let staged = staged_path(&exe);
        std::fs::write(&staged, "new version").unwrap();
        install_at(&exe, &staged, &new_key()).unwrap();
        exe
    }

    /// For a start that must not start the previous version.
    fn no_start(_: &Path) -> Result<()> {
        panic!("only a roll back starts the previous version");
    }

    /// A start of the new version that doesn't roll back, as `main` makes it.
    fn start_new(exe: &Path) -> Start {
        check_pending_at(
            exe,
            Some(new_key().as_str()),
            false,
            &settings(exe),
            no_start,
        )
    }

    /// A start of the new version that rolls back. Returns the exe it started.
    fn start_rolling_back(exe: &Path) -> Option<PathBuf> {
        let mut started = None;
        let key = new_key();
        let start = check_pending_at(exe, Some(key.as_str()), false, &settings(exe), |previous| {
            started = Some(previous.to_path_buf());
            Ok(())
        });
        assert_eq!(start, Start::RolledBack);
        started
    }

    fn starts(exe: &Path) -> u32 {
        read_pending(exe).unwrap().expect("a marker").starts
    }

    #[test]
    fn the_previous_version_stays_until_the_new_one_works() {
        let exe = installed("pending");
        assert_eq!(starts(&exe), 0);
        // Each start counts, and none removes the previous version.
        for counted in 1..=STARTS_TO_WORK {
            assert_eq!(start_new(&exe), Start::Pending);
            clean_up_at(&exe);
            assert_eq!(starts(&exe), counted);
            assert_eq!(read(&exe), "new version");
            assert_eq!(read(&exe.with_extension("old")), "running version");
        }
        std::fs::remove_dir_all(exe.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_working_start_removes_the_previous_version_and_the_marker() {
        let exe = installed("working");
        assert_eq!(start_new(&exe), Start::Pending);
        mark_working_at(&exe);
        assert!(!exe.with_extension("old").exists());
        assert!(!pending_path(&exe).exists());
        // Later starts are normal ones, of the new version.
        assert_eq!(start_new(&exe), Start::Normal);
        clean_up_at(&exe);
        assert_eq!(read(&exe), "new version");
        std::fs::remove_dir_all(exe.parent().unwrap()).unwrap();
    }

    #[test]
    fn two_failed_starts_roll_back() {
        let exe = installed("roll-back");
        // Both crashed before they counted as working.
        for _ in 0..STARTS_TO_WORK {
            assert_eq!(start_new(&exe), Start::Pending);
        }
        assert_eq!(start_rolling_back(&exe).as_deref(), Some(exe.as_path()));
        assert_eq!(read(&exe), "running version");
        assert_eq!(read(&failed_path(&exe)), "new version");
        assert!(!exe.with_extension("old").exists());
        assert!(!pending_path(&exe).exists());
        // The previous version starts as usual and removes the failed one.
        // It starts after the roll back, with the failed version aside.
        let previous = check_pending_at(&exe, Some("8.0.0+41"), true, &settings(&exe), no_start);
        assert_eq!(previous, Start::Normal);
        clean_up_at(&exe);
        assert!(!failed_path(&exe).exists());
        assert_eq!(read(&exe), "running version");
        std::fs::remove_dir_all(exe.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_roll_back_skips_the_failed_version_in_the_settings() {
        let exe = installed("skip");
        let saved = crate::config::Config {
            max_items: 123,
            ..Default::default()
        };
        saved.save(&settings(&exe)).unwrap();
        for _ in 0..STARTS_TO_WORK {
            start_new(&exe);
        }
        start_rolling_back(&exe);
        let (loaded, problem) = crate::config::Config::load(&settings(&exe));
        assert_eq!(problem, None);
        // What "Skip this version" keeps, so the update check passes it by.
        assert_eq!(loaded.skipped_update, new_key());
        assert_eq!(loaded.max_items, 123, "the other settings stay");
        std::fs::remove_dir_all(exe.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_roll_back_leaves_unreadable_settings_as_they_are() {
        let exe = installed("unreadable");
        std::fs::write(settings(&exe), "{ not settings").unwrap();
        for _ in 0..STARTS_TO_WORK {
            start_new(&exe);
        }
        start_rolling_back(&exe);
        assert_eq!(read(&settings(&exe)), "{ not settings");
        assert_eq!(read(&exe), "running version");
        std::fs::remove_dir_all(exe.parent().unwrap()).unwrap();
    }

    #[test]
    fn without_the_previous_version_the_new_one_goes_on() {
        let exe = installed("no-previous");
        for _ in 0..STARTS_TO_WORK {
            start_new(&exe);
        }
        std::fs::remove_file(exe.with_extension("old")).unwrap();
        assert_eq!(start_new(&exe), Start::Pending);
        assert_eq!(read(&exe), "new version");
        assert!(!failed_path(&exe).exists());
        assert!(pending_path(&exe).exists(), "the next start tries again");
        std::fs::remove_dir_all(exe.parent().unwrap()).unwrap();
    }

    /// As in `a_failed_install_puts_the_running_version_back`: another
    /// program holds the previous version, so it can't take the exe's name.
    #[cfg(windows)]
    #[test]
    fn a_roll_back_that_fails_halfway_puts_the_new_version_back() {
        use std::os::windows::fs::OpenOptionsExt;
        let exe = installed("roll-back-fails");
        for _ in 0..STARTS_TO_WORK {
            start_new(&exe);
        }
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(exe.with_extension("old"))
            .unwrap();
        let start = start_new(&exe);
        drop(held);
        assert_eq!(start, Start::Pending);
        assert_eq!(read(&exe), "new version");
        assert_eq!(read(&exe.with_extension("old")), "running version");
        assert!(!failed_path(&exe).exists());
        assert!(pending_path(&exe).exists(), "the next start tries again");
        std::fs::remove_dir_all(exe.parent().unwrap()).unwrap();
    }

    /// A version installed by 0.11.0, which writes no marker, counts its
    /// own starts from its first one, and rolls back like any other.
    #[test]
    fn a_version_an_older_one_installed_watches_itself() {
        let exe = running_exe("no-marker");
        std::fs::rename(&exe, exe.with_extension("old")).unwrap();
        std::fs::write(&exe, "new version").unwrap();
        let key = new_key();
        let start = |after_update| {
            check_pending_at(
                &exe,
                Some(key.as_str()),
                after_update,
                &settings(&exe),
                no_start,
            )
        };
        // Not after an update: nothing to watch.
        assert_eq!(start(false), Start::Normal);
        assert!(!pending_path(&exe).exists());
        // Its first start after the update writes the marker and counts.
        assert_eq!(start(true), Start::Pending);
        assert_eq!(starts(&exe), 1);
        clean_up_at(&exe);
        assert_eq!(read(&exe.with_extension("old")), "running version");
        // Then it goes on as an update with a marker: a second failed start,
        // and the third puts the previous version back.
        assert_eq!(start(false), Start::Pending);
        assert_eq!(start_rolling_back(&exe).as_deref(), Some(exe.as_path()));
        assert_eq!(read(&exe), "running version");
        std::fs::remove_dir_all(exe.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_left_over_marker_is_removed() {
        let exe = installed("left-over");
        // Another version runs (put back by hand, say), or a local build,
        // or the marker can't be read.
        for (key, marker) in [
            (Some("8.0.0+41"), r#"{"key":"9.0.0+42","starts":1}"#),
            (None, r#"{"key":"9.0.0+42","starts":1}"#),
            (Some("9.0.0+42"), "{"),
        ] {
            std::fs::write(pending_path(&exe), marker).unwrap();
            let start = check_pending_at(&exe, key, false, &settings(&exe), no_start);
            assert_eq!(start, Start::Normal, "{key:?} {marker}");
            assert!(!pending_path(&exe).exists(), "{key:?} {marker}");
        }
        clean_up_at(&exe);
        assert!(!exe.with_extension("old").exists());
        std::fs::remove_dir_all(exe.parent().unwrap()).unwrap();
    }
}
