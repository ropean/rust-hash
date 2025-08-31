## Project Prompt: rust-hash (Iced GUI SHA-256 hasher)

### Overview

- Goal: Desktop utility to compute SHA-256 for files with a clean, responsive UI.
- Tech: Rust, Iced (v0.12), Dark theme UI.
- Platforms: Windows primary; others untested.

### Key Features

- Drag-and-drop file hashing, browse dialog, and path input.
- Live progress in window title as percent during hashing.
- Buttons disabled while hashing; Cancel to stop and restore previous path.
- Outputs both HEX and Base64; toggle Uppercase HEX; auto-hash on selection.
- Shows meta (duration, size, throughput). Error feedback on failure.
- Window icon from env/paths with embedded fallback.
- GitHub Actions release pipeline with UPX compression.

### UI/UX Requirements

- Title: "Rust Hash256" with dynamic progress suffix when hashing.
- Controls:
  - Path input: submit triggers Start.
  - Buttons: Browse, Clear, Cancel (Cancel visible only while hashing).
  - Toggles: Uppercase HEX, Auto hash on select.
  - Outputs: SHA-256 HEX and Base64 with Copy buttons.
  - Status: Shows path, error, or meta (elapsed, size, speed).
- Disable Browse/Clear/Copy while hashing.

### Behavior & State

- Start hashing when:
  - User presses Enter in path input; or
  - Auto-hash + new path selected/pasted/browsed/dropped.
- Progress: Poll a shared atomic counter every ~100ms, update title percent.
- Cancel: Set cancel flag; stop worker; clear progress; restore previous path.
- Previous Path: Capture before path changes triggering hashing; restore on cancel.

### Architecture

- `src/main.rs`: Single-file Iced `Application` implementation.
- Concurrency: Hashing runs in a background thread. Progress tracked via `Arc<AtomicU64>`; cancellation via `Arc<AtomicBool>`; result returned via `mpsc::channel` and polled on tick.
- Subscriptions: Batch file-drop events with a periodic timer tick.
- Tokening: `token: u64` tracks current hash to ignore outdated results.

### Important Types

- `Message` variants: `PathChanged`, `BrowsePressed`, `ClearPressed`, `CancelPressed`, `CopyHex`, `CopyBase64`, `UppercaseToggled(bool)`, `AutoHashToggled(bool)`, `DroppedFile(PathBuf)`, `StartHash`, `Tick`, `Ignored`.
- `App` fields (selection): `path_input`, `hex_output`, `base64_output`, `is_hashing`, `uppercase`, `auto_hash`, `started_at`, `last_*` (elapsed/bytes/path), `prev_path_before_hash`, `progress_total`, `progress_processed`, `progress_counter`, `cancel_flag`, `worker_rx`, `worker_token`, `token`.
- `HashResult`: `hex`, `base64`, `elapsed`, `bytes`, `path`.

### Window Icon Loading Strategy

1. `APP_ICON`/`ICON` env var path.
2. `assets/app.ico` at CWD or exe-relative.
3. Embedded `assets/app.ico` written to a temp file and loaded.

### Build & Release

- Local: `build-icon.cmd` builds with `--features windows-icon`, then UPX compresses if available.
- CI: `.github/workflows/release.yml` builds on tag push, installs UPX, compresses `rust-hash.exe`, uploads artifact, creates release.

### Constraints & Style

- Rust 2021, explicit, readable code.
- Avoid deep nesting; prefer early returns.
- No inline comments for trivial code; doc comments for complex parts.
- Preserve indentation and formatting.

### Future Enhancements (Optional)

- Multiple hash algos (SHA-1/512, BLAKE3), multi-file queue, pause/resume.
- Theming options; localization; portable zip packaging.
