@echo off
setlocal enabledelayedexpansion

REM Build with Windows icon feature enabled (release)
cargo build --release --features windows-icon
if errorlevel 1 (
  echo Build failed.
  exit /b 1
)

set BIN=target\release\rust-hash.exe
if not exist "%BIN%" (
  echo Binary not found: %BIN%
  exit /b 1
)

REM Try to find UPX in PATH
where upx >nul 2>&1
if errorlevel 1 (
  echo UPX not found in PATH. Skipping compression.
  echo You can download UPX from https://github.com/upx/upx/releases
  exit /b 0
)

echo Compressing with UPX...
upx --best --lzma "%BIN%"
if errorlevel 1 (
  echo UPX compression failed.
  exit /b 1
)

echo Done. Compressed binary at %BIN%
exit /b 0


