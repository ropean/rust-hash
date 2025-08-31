@echo off
setlocal enabledelayedexpansion

echo Cleaning cargo target directory...
cargo clean
if errorlevel 1 (
  echo cargo clean failed. Continuing...
)

echo Removing temporary build files...
set TEMP_ICONS=%TEMP%\rust-hash-app.ico
if exist "%TEMP_ICONS%" del /f /q "%TEMP_ICONS%"

echo Removing leftover artifacts...
if exist target\release\rust-hash.pdb del /f /q target\release\rust-hash.pdb

echo Done.
exit /b 0


