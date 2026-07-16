# digi Grok enterprise installer — same as install.ps1 (source build).
#   irm https://raw.githubusercontent.com/DigiGoon/digi-grok-build/main/scripts/install-enterprise.ps1 | iex

$ErrorActionPreference = 'Stop'
$Ref = if ($env:DGROK_REF) { $env:DGROK_REF } else { 'main' }

if ($PSScriptRoot -and (Test-Path (Join-Path $PSScriptRoot 'install.ps1'))) {
    & (Join-Path $PSScriptRoot 'install.ps1') @args
    return
}

$url = "https://raw.githubusercontent.com/DigiGoon/digi-grok-build/$Ref/scripts/install.ps1"
Write-Host "Fetching digi install.ps1 from $url …" -ForegroundColor DarkGray
irm $url | iex
