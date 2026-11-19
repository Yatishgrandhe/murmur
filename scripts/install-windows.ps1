# Build Murmur from source on Windows, install it for the current user, and start it.
#
# Needs: Rust (https://rustup.rs), Node 18+ and Visual Studio 2022 Build Tools (C++ workload).
# Quick setup with winget:
#   winget install Rustlang.Rustup OpenJS.NodeJS.LTS
#   winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --passive"
#
# Usage (PowerShell, from the repo root):  powershell -ExecutionPolicy Bypass -File scripts\install-windows.ps1
$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

npm ci
npx tauri build --bundles nsis

$installer = Get-ChildItem "src-tauri\target\release\bundle\nsis\*-setup.exe" |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $installer) { throw "Installer not found under src-tauri\target\release\bundle\nsis" }

Get-Process murmur -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Process $installer.FullName -ArgumentList "/S" -Wait   # silent, per-user install

$exe = Join-Path $env:LOCALAPPDATA "Murmur\murmur.exe"
if (-not (Test-Path $exe)) { $exe = Join-Path $env:LOCALAPPDATA "Programs\Murmur\murmur.exe" }
Start-Process $exe
Write-Host "Murmur installed and running. Turn on Settings > Privacy & security > Speech > Online speech recognition, then tap Ctrl+Shift+Space, speak, and pause."
