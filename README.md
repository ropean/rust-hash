# rust-hash

A fast, minimal, and beautiful SHA-256 file hasher built with Rust + iced.

Features

- Drag & drop a file anywhere to hash
- Browse file dialog and manual path input
- Non-blocking, streamed hashing (1 MiB buffer) for large files
- Copy buttons for HEX and Base64
- Uppercase toggle for HEX
- Auto-hash on select, Enter-to-hash on the path input
- Clear output, elapsed time, byte size and throughput
- Dark theme, centered window on start
- Windows release builds hide the console window
- Optional Windows icon embedding via feature `windows-icon`

Requirements

- Windows 10/11 (other platforms may work but are untested here)
- Rust toolchain with cargo (MSVC recommended)

Install Rust (recommended MSVC toolchain)
Option A: Installer (GUI)

1. Go to `https://rustup.rs`
2. Download and run the installer.
3. Choose the default installation (MSVC).

Option B: Scoop

```powershell
scoop install rust-msvc
```

Then restart your terminal so `cargo` is in PATH.

Build & Run

```bat
cd rust-hash
cargo run --release
```

The optimized binary will be at `target\release\rust-hash.exe`.

Convenient cargo aliases (see `.cargo/config.toml`)

- Build with Windows icon feature: `cargo build-icon`
- Run with Windows icon feature: `cargo run-icon`

Embed a Windows .ico (optional)

- Provide an `.ico` path via environment variable, then build with the feature:

```bat
set APP_ICON=assets\app.ico
cargo build --release --features windows-icon
```

Usage

- Paste a path or click "Browse" to select a file, or drop a file anywhere in the window.
- Hashing runs automatically when a file is selected or when you press Enter in the path field.
- Use "Copy HEX" or "Copy Base64" to copy results.
- "Clear" resets inputs and outputs.

Notes

- Hashing is streamed and off the UI thread, so large files are safe.
- Base64 encodes the raw SHA-256 digest; HEX casing is configurable.
- Throughput display is approximate (uses file size and elapsed).

License
MIT
