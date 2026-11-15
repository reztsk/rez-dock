# Publie une nouvelle version : bump version (package.json, Cargo.toml, tauri.conf.json),
# commit, tag vX.Y.Z, push → GitHub Actions build et publie la release + latest.json.
# Usage : npm run release -- 1.0.1        (ou : .\scripts\release.ps1 1.0.1 "Notes")
param(
    [Parameter(Mandatory = $true, Position = 0)][string]$Version,
    [Parameter(Position = 1)][string]$Message = ''
)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)

if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw "Version invalide '$Version' (attendu : X.Y.Z)" }
if (git status --porcelain) { throw 'Arbre git non propre : commit ou stash tes changements avant de publier.' }
if (git tag --list "v$Version") { throw "Le tag v$Version existe déjà." }

$utf8 = New-Object System.Text.UTF8Encoding($false)
function Set-Text($path, $text) { [System.IO.File]::WriteAllText((Resolve-Path $path), $text, $utf8) }

# package.json
$pkg = Get-Content package.json -Raw
Set-Text package.json ([regex]::Replace($pkg, '"version":\s*"[^"]+"', "`"version`": `"$Version`"", 1))
# tauri.conf.json
$conf = Get-Content src-tauri\tauri.conf.json -Raw
Set-Text src-tauri\tauri.conf.json ([regex]::Replace($conf, '"version":\s*"[^"]+"', "`"version`": `"$Version`"", 1))
# Cargo.toml (première occurrence = [package])
$cargo = Get-Content src-tauri\Cargo.toml -Raw
Set-Text src-tauri\Cargo.toml ([regex]::Replace($cargo, '(?m)^version\s*=\s*"[^"]+"', "version = `"$Version`"", 1))

# Met à jour Cargo.lock
cargo update -p rez --manifest-path src-tauri/Cargo.toml --offline 2>$null
if ($LASTEXITCODE -ne 0) { cargo generate-lockfile --manifest-path src-tauri/Cargo.toml }

git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
$msg = if ($Message) { "release: v$Version — $Message" } else { "release: v$Version" }
git commit -m $msg
git tag -a "v$Version" -m $msg
git push origin HEAD
git push origin "v$Version"

$remote = (git remote get-url origin) -replace '\.git$', '' -replace '^git@github\.com:', 'https://github.com/'
Write-Host "`nv$Version poussée. Build en cours : $remote/actions" -ForegroundColor Green
Write-Host "Release : $remote/releases/tag/v$Version" -ForegroundColor Green
