# rust-hash

A fast, minimal, and beautiful SHA-256 file hasher built with Rust + iced.

Features
- Drag & drop a file anywhere to hash
- Browse file dialog and manual path input
- Non-blocking, streamed hashing (1 MiB buffer) for large files
- Copy buttons for HEX and Base64
- Uppercase toggle for HEX
- Auto-hash on select, Clear, elapsed time, byte size and throughput
- Unicode icons, dark theme

Requirements
- Windows 10/11 (other platforms may work but are untested here)
- Rust toolchain with cargo (MSVC recommended)

Install Rust (recommended MSVC toolchain)
Option A: Installer (GUI)
1. Go to `https://rustup.rs`
2. Download and run the installer.
3. Choose the default installation (MSVC).

Option B: PowerShell (non-interactive)
Run PowerShell as your user (no admin needed):
```powershell
Set-ExecutionPolicy Bypass -Scope Process -Force; \
  [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
  Invoke-WebRequest https://win.rustup.rs -OutFile rustup-init.exe; \
  .\rustup-init.exe -y --default-toolchain stable-x86_64-pc-windows-msvc --profile minimal
```
Then restart your terminal so `cargo` is in PATH.

Build & Run
```bat
cd rust-hash
cargo run --release
```
The optimized binary will be at `target\release\rust-hash.exe`.

Usage
- Paste a path or click "📁 Browse" to select a file.
- Or drop a file anywhere in the window.
- Click "⚙️ Hash" (or enable Auto hash).
- Use "📋 Copy HEX" or "📋 Copy Base64" to copy results.
- "🧹 Clear" resets inputs and outputs.

Notes
- Hashing is streamed and off the UI thread, so large files are safe.
- Base64 encodes the raw SHA-256 digest; HEX casing is configurable.
- Throughput display is approximate (uses file size and elapsed).

License
MIT