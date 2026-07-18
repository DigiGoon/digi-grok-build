# digi Grok (dgrok) installer for PowerShell — same shape as https://x.ai/cli/install.ps1
#
# Official Grok:  irm https://x.ai/cli/install.ps1 | iex
# digi:           irm https://raw.githubusercontent.com/DigiGoon/digi-grok-build/main/scripts/install.ps1 | iex
#
# Prebuilt from GitHub Releases (our CDN). Source build only if no asset.
#
#   irm …/install.ps1 | iex
#   .\scripts\install.ps1 -Version 0.1.0
#   .\scripts\install.ps1 -FromSource

param(
    [string]$Version = $env:DGROK_VERSION,
    [switch]$FromSource
)

$ErrorActionPreference = 'Stop'
$RepoSlug = if ($env:DGROK_REPO_SLUG) { $env:DGROK_REPO_SLUG } else { 'DigiGoon/digi-grok-build' }
$BinDir = if ($env:DGROK_BIN_DIR) { $env:DGROK_BIN_DIR } elseif ($env:GROK_BIN_DIR) { $env:GROK_BIN_DIR } else { Join-Path $env:USERPROFILE '.grok\bin' }
$DownloadDir = if ($env:DGROK_DOWNLOAD_DIR) { $env:DGROK_DOWNLOAD_DIR } else { Join-Path $env:USERPROFILE '.grok\downloads' }
$SrcDir = if ($env:DGROK_SRC) { $env:DGROK_SRC } else { Join-Path $env:USERPROFILE '.grok\src\digi-grok-build' }
$RepoRef = if ($env:DGROK_REF) { $env:DGROK_REF } else { 'main' }
$RepoUrl = if ($env:DGROK_REPO) { $env:DGROK_REPO } else { "https://github.com/$RepoSlug.git" }

if ($Version -and -not $Version.StartsWith('v')) { $Version = "v$Version" }

function Finish-Install([string]$Dest) {
    Write-Host "  Binary installed to $Dest." -ForegroundColor DarkGray
    & $Dest --version
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = @()
    if ($userPath) { $entries = $userPath -split ';' | Where-Object { $_ } }
    if ($entries -notcontains $BinDir) {
        [Environment]::SetEnvironmentVariable('Path', ((@($BinDir) + $entries) -join ';'), 'User')
        Write-Host "  Added $BinDir to your User PATH." -ForegroundColor DarkGray
    }
    if ($env:Path -notlike "*$BinDir*") { $env:Path = "$BinDir;$env:Path" }
    # Completions (best-effort)
    $compDir = Join-Path (Join-Path $env:USERPROFILE '.grok\completions') 'powershell'
    try {
        New-Item -ItemType Directory -Force -Path $compDir | Out-Null
        & $Dest completions powershell 2>$null | Set-Content (Join-Path $compDir 'dgrok.ps1') -ErrorAction SilentlyContinue
    } catch {}
    Write-Host ""
    Write-Host "Run 'dgrok' to get started!" -ForegroundColor Cyan
    Write-Host "In the TUI: /provider presets | /provider add nvidia --set-default" -ForegroundColor Cyan
}

function Install-Binary([string]$Src) {
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    $dest = Join-Path $BinDir 'dgrok.exe'
    foreach ($name in @('dgrok.exe')) {
        $d = Join-Path $BinDir $name
        try {
            Copy-Item -Force $Src $d
        } catch {
            $old = "$d.old"
            if (Test-Path $d) { Move-Item -Force $d $old -ErrorAction SilentlyContinue }
            Copy-Item -Force $Src $d
        }
    }
    Finish-Install $dest
}

function Try-Prebuilt {
    $arch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'aarch64' } else { 'x86_64' }
    $asset = "dgrok-windows-${arch}.exe"
    $api = if ($Version) {
        "https://api.github.com/repos/$RepoSlug/releases/tags/$Version"
    } else {
        "https://api.github.com/repos/$RepoSlug/releases/latest"
    }
    if ($Version) {
        Write-Host "Fetching digi dgrok $($Version.TrimStart('v'))…" -ForegroundColor DarkGray
    } else {
        Write-Host 'Fetching latest digi dgrok release…' -ForegroundColor DarkGray
    }
    try {
        $headers = @{ Accept = 'application/vnd.github+json'; 'User-Agent' = 'digi-grok-install' }
        if ($env:GITHUB_TOKEN) { $headers['Authorization'] = "Bearer $($env:GITHUB_TOKEN)" }
        $rel = Invoke-RestMethod -Uri $api -Headers $headers
    } catch {
        Write-Host "  No GitHub Release found yet." -ForegroundColor Yellow
        return $false
    }
    $item = $rel.assets | Where-Object { $_.name -eq $asset } | Select-Object -First 1
    if (-not $item) {
        Write-Host "  Release has no asset $asset" -ForegroundColor Yellow
        return $false
    }
    $tag = $rel.tag_name
    Write-Host "Installing digi dgrok $($tag.TrimStart('v')) (windows-$arch)…" -ForegroundColor Cyan
    New-Item -ItemType Directory -Force -Path $DownloadDir | Out-Null
    $tmp = Join-Path $DownloadDir "$asset.tmp"
    Write-Host "  Downloading $asset…" -ForegroundColor DarkGray
    Invoke-WebRequest -Uri $item.browser_download_url -OutFile $tmp -UseBasicParsing
    $stable = Join-Path $DownloadDir $asset
    Move-Item -Force $tmp $stable
    Install-Binary $stable
    return $true
}

function Test-DigiTree([string]$Root) {
    $cargo = Join-Path $Root 'crates\codegen\xai-grok-pager-bin\Cargo.toml'
    if (-not (Test-Path $cargo)) { return $false }
    return (Select-String -Path $cargo -Pattern 'name = "dgrok"' -Quiet)
}

function Sync-SrcCheckout([string]$Ref) {
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) { throw 'git required for source install' }
    New-Item -ItemType Directory -Force -Path (Split-Path $SrcDir -Parent) | Out-Null
    if (-not (Test-Path (Join-Path $SrcDir '.git'))) {
        Write-Host "Cloning $RepoUrl ($Ref) → $SrcDir …" -ForegroundColor DarkGray
        if (Test-Path $SrcDir) { Remove-Item -Recurse -Force $SrcDir }
        try {
            git clone --depth 1 --branch $Ref $RepoUrl $SrcDir
        } catch {
            git clone --depth 1 $RepoUrl $SrcDir
        }
        if (-not (Test-Path (Join-Path $SrcDir '.git'))) { throw 'git clone failed' }
    }

    Write-Host "Updating source at $SrcDir to $Ref …" -ForegroundColor DarkGray
    Push-Location $SrcDir
    try {
        git remote set-url origin $RepoUrl 2>$null
        $tip = $null
        git fetch --force --depth 1 origin "+refs/heads/${Ref}:refs/remotes/origin/${Ref}" 2>$null
        if ($LASTEXITCODE -eq 0) {
            $tip = "origin/$Ref"
        } else {
            git fetch --force --depth 1 origin "+refs/tags/${Ref}:refs/tags/${Ref}" 2>$null
            if ($LASTEXITCODE -eq 0) {
                $tip = "refs/tags/$Ref"
            } else {
                git fetch --force --depth 1 origin $Ref 2>$null
                if ($LASTEXITCODE -eq 0) { $tip = 'FETCH_HEAD' }
            }
        }
        if (-not $tip) {
            Write-Host 'git fetch failed; recloning…' -ForegroundColor Yellow
            Pop-Location
            Remove-Item -Recurse -Force $SrcDir
            git clone --depth 1 --branch $Ref $RepoUrl $SrcDir
            if (-not (Test-Path (Join-Path $SrcDir '.git'))) { throw 'git clone failed' }
            Push-Location $SrcDir
            $tip = 'HEAD'
        }
        if ($tip -ne 'HEAD') {
            git checkout -B $Ref $tip 2>$null
            if ($LASTEXITCODE -ne 0) { git checkout --detach $tip }
            git reset --hard $tip
            if ($LASTEXITCODE -ne 0) { throw "git reset --hard $tip failed" }
        }
        git clean -fd -e target 2>$null
        $rev = (git rev-parse --short HEAD 2>$null)
        Write-Host "  source revision: $rev" -ForegroundColor DarkGray
    } finally {
        Pop-Location
    }
}

function Install-FromSource {
    Write-Host 'No prebuilt release — building from source…' -ForegroundColor Yellow
    $repoRoot = $null
    $forceRemote = $env:DGROK_FORCE_REMOTE_SRC -eq '1'
    if (-not $forceRemote -and $PSScriptRoot) {
        $cand = Resolve-Path (Join-Path $PSScriptRoot '..') -ErrorAction SilentlyContinue
        if ($cand -and (Test-DigiTree $cand.Path)) {
            $repoRoot = $cand.Path
            Write-Host "Building from local digi tree: $repoRoot" -ForegroundColor DarkGray
            Write-Host '  (local git tree — not auto-pulling; git pull yourself or set DGROK_FORCE_REMOTE_SRC=1)' -ForegroundColor DarkGray
        }
    }
    if (-not $repoRoot) {
        $syncRef = $RepoRef
        if ($Version) { $syncRef = $Version }
        Sync-SrcCheckout $syncRef
        $repoRoot = $SrcDir
    }
    if (-not (Test-DigiTree $repoRoot)) {
        throw "Not digi-grok-build (expected dgrok binary package): $repoRoot"
    }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw 'cargo not found — install from https://rustup.rs'
    }
    $sourceRev = (git -C $repoRoot rev-parse --short HEAD 2>$null)
    if (-not $sourceRev) { throw 'cannot resolve source revision' }
    Write-Host "Building dgrok (release) in $repoRoot …" -ForegroundColor DarkGray
    Push-Location $repoRoot
    try {
        $env:DGROK_BUILD_COMMIT = $sourceRev
        & cargo build -p xai-grok-pager-bin --release
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed: $LASTEXITCODE" }
    } finally { Pop-Location }
    $src = Join-Path $repoRoot 'target\release\dgrok.exe'
    if (-not (Test-Path $src)) {
        $src = Join-Path $repoRoot 'target\release\dgrok'
    }
    if (-not (Test-Path $src)) { throw "binary not found under target/release/dgrok*" }
    $builtVersion = (& $src --version 2>$null) -join ''
    if ($LASTEXITCODE -ne 0 -or -not $builtVersion.Contains("($sourceRev)")) {
        throw "built binary does not match source revision ${sourceRev}: $builtVersion"
    }
    Write-Host "  verified binary revision: $sourceRev" -ForegroundColor DarkGray
    Install-Binary $src
}

Write-Host 'digi Grok CLI installer' -ForegroundColor Cyan

if ($FromSource -or $env:DGROK_FROM_SOURCE -eq '1') {
    Install-FromSource
    return
}
if (Try-Prebuilt) { return }
if ($env:DGROK_NO_SOURCE -eq '1') {
    throw 'No GitHub Release binary and DGROK_NO_SOURCE=1'
}
Install-FromSource
