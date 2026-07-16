# Build digi-grok-build from source and install the CLI as `dgrok`.
#
# Usage (from repo root, PowerShell):
#   .\scripts\install-from-source.ps1
#   .\scripts\install-from-source.ps1 -Release
#   $env:DGROK_BIN_DIR = "$env:USERPROFILE\.local\bin"; .\scripts\install-from-source.ps1

param(
    [switch]$Release
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$BinDir = if ($env:DGROK_BIN_DIR) {
    $env:DGROK_BIN_DIR
} elseif ($env:GROK_BIN_DIR) {
    $env:GROK_BIN_DIR
} else {
    Join-Path $env:USERPROFILE '.grok\bin'
}

$ProfileName = if ($Release) { 'release' } else { 'debug' }
$CargoArgs = @('build', '-p', 'xai-grok-pager-bin')
if ($Release) { $CargoArgs += '--release' }

Push-Location $RepoRoot
try {
    Write-Host "Building dgrok from source ($ProfileName)..." -ForegroundColor Cyan
    & cargo @CargoArgs
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

    $Src = Join-Path $RepoRoot "target\$ProfileName\dgrok.exe"
    if (-not (Test-Path $Src)) {
        throw "Expected binary not found at $Src"
    }

    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    $Dest = Join-Path $BinDir 'dgrok.exe'
    Copy-Item -Force $Src $Dest
    Write-Host "Installed: $Dest" -ForegroundColor Green
    & $Dest --version

    $UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($UserPath -notlike "*$BinDir*") {
        Write-Host "Add to User PATH (or current session):" -ForegroundColor Yellow
        Write-Host "  `$env:Path = `"$BinDir;`$env:Path`""
    }
    Write-Host "Run: dgrok" -ForegroundColor Cyan
} finally {
    Pop-Location
}
