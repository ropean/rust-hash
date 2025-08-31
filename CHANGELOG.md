# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2025-08-31

### Added

- Progress percentage shown in window title while hashing.
- Cancel button to stop hashing; restores previous path when possible.
- Robust window icon loading with embedded fallback.
- GitHub Actions release workflow (tag-based) that builds Windows binary and compresses it with UPX.
- `build-icon.cmd` script to build with icon and compress with UPX locally.
- `AI_PROMPT.md` documenting project structure and requirements.

### Changed

- Disable Browse/Clear/Copy buttons while hashing.
- Browse dialog opens in last-used directory when available.
- Internal refactor: background hashing thread with progress/cancel tracking and periodic UI updates.

### Fixed

- Title icon not displaying in some environments (added multiple fallbacks).
- Build warnings from unused code and message variants.

[0.2.0]: https://github.com/your-org/rust-hash/releases/tag/v0.2.0

## [0.1.0] - 2025-08-29

### Added

- Rust + iced GUI for SHA-256 hashing.
- Drag-and-drop support and file browser dialog.
- Streaming SHA-256 (1 MiB buffer) off the UI thread for large files.
- Outputs: HEX (with uppercase toggle) and Base64.
- Controls: Copy HEX, Copy Base64, Clear.
- Auto-hash on select and Enter-to-hash on the path input.
- Metadata display: elapsed time, file size, and approximate throughput.
- Centered window on start.
- Windows release builds hide the console window.
- Optional Windows icon embedding using feature `windows-icon` (via `build.rs` + `winres`).
  - Set an icon by defining `APP_ICON` (e.g., `assets\\app.ico`) and building with `--features windows-icon`.
- Cargo aliases for convenience in `.cargo/config.toml`:
  - `cargo build-icon` → `cargo build --release --features windows-icon`
  - `cargo run-icon` → `cargo run --release --features windows-icon`
- .gitignore for build artifacts, OS files, and editor settings.

### Changed

- UI polish: plain-text labels (removed emojis), improved spacing.
- Removed dedicated "Hash" button; hashing is automatic or via Enter.
- Copy buttons align consistently when a value exists.
- Default window size is 900×560; minimum is 900×420.

### Removed

- npm scripts (`package.json`) were removed; use cargo aliases instead.

[0.1.0]: https://github.com/your-org/rust-hash/releases/tag/v0.1.0
