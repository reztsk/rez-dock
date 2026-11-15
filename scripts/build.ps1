# Build local signé : produit l'installeur NSIS dans src-tauri\target\release\bundle\nsis\
# Usage : npm run app:build
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)

$key = Join-Path $env:USERPROFILE '.tauri\rez.key'
if (-not (Test-Path $key)) {
    Write-Host "Clé de signature absente ($key). Génération..." -ForegroundColor Yellow
    npx tauri signer generate --ci -p '""' -w $key
}
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $key -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''

npx tauri build
if ($LASTEXITCODE -ne 0) { throw "Build échoué" }

$out = Get-ChildItem 'src-tauri\target\release\bundle\nsis\*.exe' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
Write-Host "`nInstalleur : $($out.FullName)" -ForegroundColor Green
