# Security

## Reporting a problem

Please don't open a public issue. Report it privately through
[GitHub's private vulnerability reporting](https://github.com/NourEldin-Osama/clipboard-manager/security/advisories/new).
You'll get a reply as soon as possible, and credit in the fix if you'd like.

## Supported versions

Only the latest release gets fixes. The app updates itself, so most people are on it within a day of a release.

## How the app protects your data

- **Offline by design.** Nothing you copy leaves your PC. Network access is limited to model downloads you start, the
  one-time OCR model download, the update check (which only fetches `updater.json`), and an embedding server only if
  you configure one yourself.
- **Secrets aren't recorded** by default: copies that look like passwords, API keys, tokens or private keys are
  skipped, as are copies other apps mark as sensitive.
- **Local storage:** history, images and settings stay in `%APPDATA%\clipboard-manager\data`.
- **API keys** for embedding servers are encrypted in `config.json` with Windows DPAPI, so only your Windows account
  on that PC can read them ([`src/secure.rs`](src/secure.rs)). A key copied to another user or PC has to be entered again.

## How updates are verified

Each release's `.exe` is signed by the release workflow with an Ed25519 key kept as a GitHub Actions secret
(`UPDATE_SIGNING_KEY`). `updater.json` carries its SHA-256 and signature. The app installs a download only if both
match the public key compiled into it ([`src/update.rs`](src/update.rs)); anything else is refused and the running
version is kept.
