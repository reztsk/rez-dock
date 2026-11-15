# Création du dépôt GitHub PUBLIC + secrets de signature + premier push + première release.
# Prérequis : gh CLI connecté (gh auth login).
# Usage : .\scripts\setup-github.ps1 [-Repo rez-dock] [-Owner tonPseudo]
param(
    [string]$Repo = 'rez-dock',
    [string]$Owner = ''
)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { throw 'gh CLI introuvable : https://cli.github.com' }
gh auth status | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Connecte-toi : gh auth login' }
if (-not $Owner) { $Owner = (gh api user --jq .login).Trim() }
$full = "$Owner/$Repo"

# 1. Endpoint de mise à jour → ce dépôt
$utf8 = New-Object System.Text.UTF8Encoding($false)
$confPath = Resolve-Path 'src-tauri\tauri.conf.json'
$conf = [System.IO.File]::ReadAllText($confPath)
$conf = [regex]::Replace($conf, 'https://github\.com/[^/"]+/[^/"]+/releases/latest/download/latest\.json', "https://github.com/$full/releases/latest/download/latest.json")
[System.IO.File]::WriteAllText($confPath, $conf, $utf8)
$ts = Resolve-Path 'src\settings\main.ts'
$t = [System.IO.File]::ReadAllText($ts)
$t = [regex]::Replace($t, "const REPO_URL = '[^']*';", "const REPO_URL = 'https://github.com/$full';")
[System.IO.File]::WriteAllText($ts, $t, $utf8)

# 2. Clé de signature (hors dépôt) + clé publique dans tauri.conf.json
$key = Join-Path $env:USERPROFILE '.tauri\rez.key'
if (-not (Test-Path $key)) {
    New-Item -ItemType Directory -Force (Split-Path $key) | Out-Null
    npx tauri signer generate --ci -p '""' -w $key
}
$pub = (Get-Content "$key.pub" -Raw).Trim()
$conf = [System.IO.File]::ReadAllText($confPath)
$conf = [regex]::Replace($conf, '"pubkey":\s*"[^"]*"', "`"pubkey`": `"$pub`"")
[System.IO.File]::WriteAllText($confPath, $conf, $utf8)

# 3. Dépôt git local
if (-not (Test-Path .git)) { git init -b main }
git add -A
git diff --cached --quiet
if ($LASTEXITCODE -ne 0) { git commit -m 'REZ : dock Expo-Glass pour Windows' }

# 4. Dépôt GitHub public
gh repo view $full 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) {
    gh repo create $full --public --description 'REZ — Dock Expo-Glass pour Windows (Tauri/Rust)' --source . --remote origin
} elseif (-not (git remote 2>$null | Select-String -Quiet '^origin$')) {
    git remote add origin "https://github.com/$full.git"
}

# 5. Secrets de signature pour GitHub Actions
Get-Content $key -Raw | gh secret set TAURI_SIGNING_PRIVATE_KEY --repo $full
# Clé sans mot de passe : TAURI_SIGNING_PRIVATE_KEY_PASSWORD reste vide (non défini = vide).

# 6. Push + première release
git push -u origin main
$version = ((Get-Content package.json -Raw | ConvertFrom-Json).version)
if (-not (git tag --list "v$version")) {
    git tag -a "v$version" -m "release: v$version"
}
git push origin "v$version"

Write-Host "`nDépôt : https://github.com/$full" -ForegroundColor Green
Write-Host "Build : https://github.com/$full/actions" -ForegroundColor Green
Write-Host "Lien à partager (après le build) : https://github.com/$full/releases/latest" -ForegroundColor Cyan
Write-Host "IMPORTANT : sauvegarde $key — sans elle, plus aucune mise à jour possible." -ForegroundColor Yellow
