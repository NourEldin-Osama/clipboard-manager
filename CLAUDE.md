# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Offline clipboard manager for Windows (also builds on Linux/macOS), pure Rust: egui/eframe UI, redb storage,
on-device OCR (ocrs), and semantic search with a built-in SigLIP model (candle) or an OpenAI-compatible embedding server.

## Commands

```bash
cargo run                                   # debug build; model/image crates are still optimised (see Cargo.toml)
cargo test --config ci/fast-build.toml      # what CI runs; much faster from a cold cache
cargo test search::tests::hybrid_is_ordered_by_relevance   # one test
cargo clippy --all-targets --config ci/fast-build.toml
cargo check --target x86_64-pc-windows-gnu  # Windows-only code paths from Linux (needs mingw-w64 for ring's build script)
prek run --all-files                        # every git hook: file checks, fmt, clippy -D warnings (see CONTRIBUTING.md)
cargo deny check -W unmaintained            # licenses, advisories, duplicates, crate sources (deny.toml)
```

- Three tests are `#[ignore]` and need local assets: `OCR_DIR=… cargo test -- --ignored ocr_reads_text`,
  `SIGLIP_DIR=…` (`text_finds_matching_image`), `EMBED_URL=…` (`server_lists_models_and_embeds`).
- When the repo lives on Google Drive (as in Colab), build with `CARGO_TARGET_DIR` pointing outside it.
- `CLIPBOARD_MANAGER_HOME=<dir>` gives a throwaway data folder (config, history, models) for testing.
- `--profile dist` (thin LTO) is the shipping build. Don't build the release `.exe` locally; CI publishes it.
- The UI can be checked under Xvfb (`xclip` to feed the clipboard, `xdotool` to drive it, `import` to screenshot);
  windows need `xdotool windowfocus` before key presses register.

## Working on Windows

- Needs Rust 1.95 or newer (egui 0.37 requires it; CI uses the newest stable) on the MSVC toolchain, plus the Visual
  Studio C++ build tools for the linker.
- The commands above work in PowerShell. Set variables first, for example
  `$env:OCR_DIR = "..."; cargo test -- --ignored ocr_reads_text`.
- If the released app is installed on the same PC, a dev build can disturb it:
  - Without `CLIPBOARD_MANAGER_HOME`, it opens the installed app's data folder (`%APPDATA%\clipboard-manager\data`).
  - Start at sign-in is one per-user `Run` registry value named "Clipboard Manager". A release build corrects its
    path at start (`autostart::refresh`, which never turns it back on); a dev build leaves it alone. Switching the
    setting in a dev build's Settings still points it at the dev exe; starting the installed app once fixes that.
  - Two apps can't register the same global shortcut. Quit the installed app before testing shortcuts.

A throwaway data folder for a dev build:

```powershell
$env:CLIPBOARD_MANAGER_HOME = "$env:TEMP\clipboard-manager-dev"
cargo run
```

## Architecture

One egui `App` (`src/app.rs`) owns all state; background threads talk to it over `mpsc` channels and wake it with
`ctx.request_repaint()`:

- **Clipboard watcher** (`clipboard.rs`) polls the clipboard and sends `Captured` (text/image/files, or `Skipped` for
  secrets and app-marked sensitive data; the secret rules are in `privacy.rs`).
- **Worker** (`worker.rs`) owns the OCR engine and the embedder; takes `Job`s (OCR, embed, set backend, query vector)
  and returns `Event`s. Embeddings are stored per model key, so switching models re-indexes without losing others.
- **Image loader** (`images.rs`) decodes thumbnails/previews off the UI thread.
- **Background** (`app/background.rs`): saving a copied image as PNG and copying an item back run in order on one
  thread; each import reads its files on its own thread and sends them in batches. Results come back as `Done`
  messages, and `App::finish` saves them through the `Library`.
- **Store** (`store.rs`): redb; items as JSON (new fields need `#[serde(default)]`), images as PNG files beside it. On Windows
  both are encrypted with the history's key (`vault.rs`, sealed with DPAPI); read and write images only through
  `store.images` (`ImageFiles`), never with `image::open` on an item's path.
- **Library** (`library.rs`): the history in memory (items, their lower-cased search text, the current model's vectors)
  kept in step with the store. Every change to the history goes through it; it has no UI code and its own tests.
  Items sit behind `Arc`, so `snapshot()` hands a search the whole history without copying texts or vectors.
- **Search** (`search.rs`): keyword score plus cosine similarity; text and image similarities are each scaled by their
  best match before merging (the "modality gap"), Smart adds a keyword boost, and `order()` applies the Sort by choice.
  It runs on the `Searcher` thread; only the newest search's results are shown. Results are `Hit`s by item id, so
  after removing items `App` also takes them out of `results` (`drop_removed`).

`App::logic` runs every frame **even while the window is hidden** (tray mode), so channel draining, tray commands,
hide/show and update checks live there (`window_logic`, `update_logic`, `handle_events`); `App::ui` only draws.
`src/app/view/` is the main window (`mod.rs` wires `top_bar`, `list`, `grid`, `details`, `chrome`, `keyboard`), `src/app/settings/` the settings (one file per page, `mod.rs` the frame); settings apply live except the search model
(Apply button) and global hotkeys. `Config` (`config.rs`) is JSON with `#[serde(default)]`, so new fields need defaults.

Other modules: `update.rs` (self-update), `tray.rs`, `autostart.rs`, `i18n.rs`, `embed/` (SigLIP via candle, HTTP
server client with optional `llama-server` launcher), `import.rs` (files/folders as items), `models.rs` (downloads).

## Conventions that aren't obvious from one file

- **No UI text in the code.** Every text is a key in `locales/en/main.ftl` and `locales/ar/main.ftl` (Project
  Fluent): `t!("details-copy")`, `t!("toast-cleared", count = n)`. Errors use `log_error!`, which logs the English text
  and returns the text to show. Keys start with the part of the window; whole sentences, never pieced together; plural
  forms live in the files. Tests check that both languages have the same keys and values, and that code and files
  match. See `docs/TRANSLATING.md`.
- **Right-to-left:** egui shapes Arabic but lays runs out left to right, so text goes through `src/app/bidi.rs`. In
  UI code use the helpers in `view/widgets.rs` — `txt`, `lbl`, `tip`, `row`, `far`, `row_wrapped`, `paint_text`, `flip`,
  `start_anchor` — instead of `ui.label`/`ui.horizontal`/`painter.text`, and mirror hand-placed rects with `flip`.
- **Icons** are SVGs in `assets/icons` (Fluent UI System Icons and Material Symbols, chosen by the maintainer), white,
  tinted at runtime via `icons::paint`. Never hand-draw an icon or use emoji/font glyphs; show options from popular
  icon sets and let the maintainer pick. The app icon (`assets/app-icon/app.ico`, embedded by `build.rs`) is not chosen yet.
- Panel frames must use opaque fills (nothing is painted beneath a panel; translucent fills show black).
- Commit messages: no `Co-Authored-By` trailers (the prek `commit-msg` hook rejects them).
- A breaking change to settings or stored data ships with a migration in the next version; remove the migration in
  the version after that (D33 in `docs/WORK-PLAN.md`).

## Releases and CI

- `main` is protected: changes go through a pull request, and the `check` job (`.github/workflows/checks.yml`: tests and
  clippy on Windows with `ci/fast-build.toml`) must pass. Checks also run on pushes to `main` to keep the cache warm.
- `dependencies.yml` runs cargo-deny (`deny.toml`) when the crates change and every Monday; it is not a required
  check. An advisory that can't affect the app goes in `advisories.ignore` with the reason.
- Every push to `main` that changes code runs `build-windows.yml`, which builds with `BUILD_NUMBER`, signs the release
  line (version, build, size, SHA-256, links) with the Ed25519 `UPDATE_SIGNING_KEY` secret, and publishes release `v<Cargo version>-build.<run>` with the exe,
  `updater.json`, a Scoop manifest and winget manifests (schema 1.12.0). Docs-only pushes don't release.
- Self-update (`update.rs`) reads `releases/latest/download/updater.json`, verifies SHA-256 and the signature against
  `PUBLIC_KEY`, swaps the exe (running one renamed `.old`) and relaunches with `--after-update`. Local builds have no
  `BUILD_NUMBER` and never update. `CLIPBOARD_MANAGER_UPDATE_URL` points the check at a test server.
- Bump the version in `Cargo.toml` for a release worth noticing (patch for fixes, minor for features).
- Release notes come from `CHANGELOG.md`: the release job puts the section for the `Cargo.toml` version at the top.
  Add the notes there before a push that makes a release.
