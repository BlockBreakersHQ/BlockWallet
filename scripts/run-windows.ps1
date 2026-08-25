# GTK on this machine lives in MSYS2 mingw64. Plain `cargo run` uses MSVC and has no pkg-config.
$ErrorActionPreference = "Stop"

$mingw = "C:\msys64\mingw64"
if (-not (Test-Path "$mingw\bin\pkg-config.exe")) {
    Write-Error "MSYS2 GTK not found at $mingw. Install mingw-w64-x86_64-gtk4, libadwaita, pkgconf, and gcc."
}

$env:PATH = "$mingw\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:PKG_CONFIG_PATH = "$mingw\lib\pkgconfig"
$env:XDG_DATA_DIRS = "$mingw\share"
$env:GSETTINGS_SCHEMA_DIR = "$mingw\share\glib-2.0\schemas"
if (-not $env:GSK_RENDERER) { $env:GSK_RENDERER = "cairo" }
if (-not $env:BLOCKWALLET_HOME) {
    $env:BLOCKWALLET_HOME = Join-Path $env:TEMP "blockwallet-dev"
}

Set-Location (Split-Path -Parent $PSScriptRoot)
cargo +stable-x86_64-pc-windows-gnu run @args
