# Contributing

Thanks for helping! Bug reports, ideas and pull requests are all welcome.

## Reporting

- **Bugs and ideas:** open an [issue](https://github.com/NourEldin-Osama/clipboard-manager/issues/new/choose). For bugs, include the version (*Settings → General → Updates*) and the relevant lines of the log (*Settings → Data & privacy → Log file*).
- **Security problems:** report them privately; see [SECURITY.md](SECURITY.md).

## Building

You need [Rust](https://rustup.rs) (stable).

```bash
cargo run              # debug build; model and image crates are still optimised
cargo test
cargo clippy
```

Builds made on your own PC have no build number and never update themselves. The Windows release is built by
GitHub Actions on every push to `main` (see the README for cross-compiling from Linux). If the repo lives on a synced
drive, keep the build cache elsewhere: `CARGO_TARGET_DIR=/some/local/dir cargo build`.

## Pull requests

- Branch from `main` and open a pull request against it; checks (tests, lint) run on every pull request.
- Keep changes focused, and match the surrounding code's style and comment density.
- For UI changes, add before/after screenshots.
- Merging into `main` publishes a release automatically, so `main` should always be shippable.

## Translations

Interface text is written in English at the call site and translated through `tr("…")` or
`trf("… {name} …", &[("name", &value)])`. Arabic lives in [`src/i18n/ar.rs`](src/i18n/ar.rs), keyed by the English text.
When you add or change a visible string, add or update its Arabic entry; a test checks that placeholders match and
keys aren't duplicated. Arabic text is laid out right to left by the helpers in `src/app/bidi.rs`; use `txt`, `lbl`,
`row` and `far` from `src/app/view.rs` instead of `ui.label`/`ui.horizontal` so new UI mirrors correctly.

## Icons

Icons come from popular open icon sets (currently Fluent UI System Icons and Material Symbols, see
[`assets/icons/LICENSES.md`](assets/icons/LICENSES.md)); they are never drawn by hand. To add or change one, open an
issue or pull request showing the options side by side; the maintainer picks. Icons are white SVGs in `assets/icons`,
tinted at runtime (`src/app/icons.rs`).

## License

By contributing you agree that your work is licensed under the project's MIT or Apache-2.0 licenses.
