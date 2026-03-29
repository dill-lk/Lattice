# Lattice Blockchain — One-Click Installer for Windows (PowerShell)
# Source: https://github.com/dill-lk/Lattice/releases
#
# Usage:
#   irm https://raw.githubusercontent.com/dill-lk/Lattice/main/install.ps1 | iex
#   .\install.ps1 [-InstallDir <path>] [-Uninstall]
#
# If you get an execution-policy error, run this first (once, as your user):
#   Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser

[CmdletBinding()]
param(
    [string] $InstallDir = "$env:USERPROFILE\.local\bin",
    [switch] $Uninstall
)

$ErrorActionPreference = "Stop"

# Enforce TLS 1.2+ — required on older Windows 10 builds
[Net.ServicePointManager]::SecurityProtocol =
    [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13

# ── Configuration ─────────────────────────────────────────────────────────────
if ($env:LATTICE_BIN_DIR) { $InstallDir = $env:LATTICE_BIN_DIR }

$GithubRepo  = "dill-lk/Lattice"
$GithubApi   = "https://api.github.com/repos/$GithubRepo/releases/latest"
$AssetName   = "lattice-windows-amd64.zip"
$ConfigDir   = "$env:USERPROFILE\.lattice\config"
$DataDir     = "$env:USERPROFILE\.lattice\data"
$Binaries    = @("lattice-node.exe", "lattice-cli.exe", "lattice-miner.exe")

# ── Helpers ───────────────────────────────────────────────────────────────────
function Print-Header {
    Write-Host ""
    Write-Host "╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║                                                          ║" -ForegroundColor Cyan
    Write-Host "║          🚀 LATTICE BLOCKCHAIN INSTALLER 🚀             ║" -ForegroundColor Green
    Write-Host "║                                                          ║" -ForegroundColor Cyan
    Write-Host "║     Quantum-Resistant Blockchain · GitHub Releases      ║" -ForegroundColor Cyan
    Write-Host "║                                                          ║" -ForegroundColor Cyan
    Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
    Write-Host ""
}

function Write-Step    { param([string]$M) Write-Host "==> " -ForegroundColor Blue -NoNewline; Write-Host $M -ForegroundColor Green }
function Write-Info    { param([string]$M) Write-Host "i  " -ForegroundColor Cyan   -NoNewline; Write-Host $M }
function Write-Ok      { param([string]$M) Write-Host "ok " -ForegroundColor Green  -NoNewline; Write-Host $M }
function Write-Warn    { param([string]$M) Write-Host "!  " -ForegroundColor Yellow -NoNewline; Write-Host $M }
function Write-Err     { param([string]$M) Write-Host "x  " -ForegroundColor Red    -NoNewline; Write-Host $M }

# ── Uninstall ─────────────────────────────────────────────────────────────────
function Invoke-Uninstall {
    Write-Step "Removing Lattice binaries from $InstallDir..."
    $removed = 0
    foreach ($bin in $Binaries) {
        $path = Join-Path $InstallDir $bin
        if (Test-Path $path) {
            Remove-Item $path -Force
            Write-Ok "Removed $bin"
            $removed++
        }
    }
    if ($removed -eq 0) {
        Write-Warn "No Lattice binaries found in $InstallDir"
    } else {
        Write-Ok "Uninstall complete ($removed binaries removed)"
    }
    Write-Info "Config and data in $env:USERPROFILE\.lattice were left intact."
    exit 0
}

if ($Uninstall) { Invoke-Uninstall }

# ── Fetch latest release ───────────────────────────────────────────────────────
function Get-LatestRelease {
    Write-Step "Fetching latest release from GitHub..."

    $release = Invoke-RestMethod -Uri $GithubApi -UseBasicParsing

    $script:ReleaseTag = $release.tag_name
    if (-not $script:ReleaseTag) {
        Write-Err "Could not get release info. Check your internet connection."
        Write-Info "Releases: https://github.com/$GithubRepo/releases"
        exit 1
    }

    $asset = $release.assets | Where-Object { $_.name -eq $AssetName } | Select-Object -First 1
    if (-not $asset) {
        Write-Err "No asset named '$AssetName' found in release $script:ReleaseTag."
        Write-Info "See: https://github.com/$GithubRepo/releases"
        exit 1
    }

    $script:DownloadUrl = $asset.browser_download_url
    Write-Ok "Latest release: $script:ReleaseTag"
}

# ── Download and install ───────────────────────────────────────────────────────
function Install-Binaries {
    Write-Step "Downloading $AssetName..."

    $tmpDir  = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $tmpDir | Out-Null
    $archive = Join-Path $tmpDir $AssetName

    $ProgressPreference = 'SilentlyContinue'
    Invoke-WebRequest -Uri $script:DownloadUrl -OutFile $archive -UseBasicParsing
    $ProgressPreference = 'Continue'
    Write-Ok "Download complete"

    Write-Step "Installing binaries to $InstallDir..."
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    Expand-Archive -Path $archive -DestinationPath $tmpDir -Force

    $installed = 0
    foreach ($bin in $Binaries) {
        # Support both flat archives and archives with a subdirectory
        $src = Get-ChildItem -Path $tmpDir -Filter $bin -Recurse -ErrorAction SilentlyContinue |
               Select-Object -First 1
        if ($src) {
            Copy-Item $src.FullName (Join-Path $InstallDir $bin) -Force
            Write-Ok "Installed $bin"
            $installed++
        } else {
            Write-Warn "$bin not found in archive (skipped)"
        }
    }

    Remove-Item -Recurse -Force $tmpDir

    if ($installed -eq 0) {
        Write-Err "No binaries were installed. The release asset may be empty."
        exit 1
    }
    Write-Ok "$installed binaries installed to $InstallDir"
}

# ── Add InstallDir to PATH ────────────────────────────────────────────────────
function Set-UserPath {
    Write-Step "Setting up PATH..."

    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($current -like "*$InstallDir*") {
        Write-Ok "PATH already includes $InstallDir"
        return
    }

    [Environment]::SetEnvironmentVariable("Path", "$current;$InstallDir", "User")
    $env:PATH = "$env:PATH;$InstallDir"
    Write-Ok "Added $InstallDir to your user PATH"
    Write-Warn "Restart your terminal (or open a new PowerShell window) for the PATH change to take effect"
}

# ── Create default configuration ──────────────────────────────────────────────
function New-DefaultConfig {
    Write-Step "Creating default configuration..."

    foreach ($dir in @($ConfigDir, $DataDir)) {
        if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
    }

    $cfg = Join-Path $ConfigDir "node.toml"
    if (Test-Path $cfg) {
        Write-Info "Config already exists at $cfg — skipping"
        return
    }

    @"
# Lattice Node Configuration
# Generated by the installer — edit as needed.

[network]
listen_addr     = "/ip4/0.0.0.0/tcp/30303"
bootstrap_nodes = []
max_peers       = 50

[consensus]
mining_threads = 0
difficulty     = 1000000

[rpc]
listen_addr = "127.0.0.1:8545"
enabled     = true

[storage]
db_path    = "$DataDir"
cache_size = 256
"@ | Out-File -FilePath $cfg -Encoding utf8

    Write-Ok "Config written to $cfg"
}

# ── Desktop shortcuts (best-effort) ───────────────────────────────────────────
function New-Shortcuts {
    Write-Step "Creating desktop shortcuts..."
    try {
        $desktop = [Environment]::GetFolderPath("Desktop")
        if (-not $desktop -or -not (Test-Path $desktop)) {
            Write-Warn "Desktop folder not found — skipping shortcuts"
            return
        }

        $shell = New-Object -ComObject WScript.Shell

        $sc = $shell.CreateShortcut("$desktop\Lattice Node.lnk")
        $sc.TargetPath       = Join-Path $InstallDir "lattice-node.exe"
        $sc.WorkingDirectory = "$env:USERPROFILE\.lattice"
        $sc.Description      = "Lattice Blockchain Node"
        $sc.Save()

        $sc = $shell.CreateShortcut("$desktop\Lattice CLI.lnk")
        $sc.TargetPath       = "powershell.exe"
        $sc.Arguments        = "-NoExit -Command `"Set-Location '$env:USERPROFILE\.lattice'; Write-Host 'Lattice CLI ready — type lattice-cli --help' -ForegroundColor Green`""
        $sc.WorkingDirectory = "$env:USERPROFILE\.lattice"
        $sc.Description      = "Lattice Command-Line Interface"
        $sc.Save()

        Write-Ok "Shortcuts created on Desktop"
    } catch {
        Write-Warn "Could not create desktop shortcuts: $_"
    }
}

# ── Completion banner ──────────────────────────────────────────────────────────
function Print-Completion {
    Write-Host ""
    Write-Host "╔══════════════════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "║        ✅  LATTICE INSTALLATION COMPLETE  ✅            ║" -ForegroundColor Green
    Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Green
    Write-Host ""
    Write-Host "📍 Installation summary:" -ForegroundColor Cyan
    Write-Host "   Version:  $script:ReleaseTag"
    Write-Host "   Binaries: $InstallDir"
    Write-Host "   Config:   $ConfigDir"
    Write-Host "   Data:     $DataDir"
    Write-Host ""
    Write-Host "🚀 Quick start:" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "   1. Create a wallet:" -ForegroundColor Yellow
    Write-Host "      lattice-cli wallet create" -ForegroundColor Blue
    Write-Host ""
    Write-Host "   2. Get your wallet address:" -ForegroundColor Yellow
    Write-Host "      lattice-cli wallet address" -ForegroundColor Blue
    Write-Host ""
    Write-Host "   3. Start the node:" -ForegroundColor Yellow
    Write-Host "      lattice-node" -ForegroundColor Blue
    Write-Host ""
    Write-Host "   4. Check node status:" -ForegroundColor Yellow
    Write-Host "      lattice-cli node status" -ForegroundColor Blue
    Write-Host ""
    Write-Host "   5. Start mining (replace YOUR_ADDRESS with your wallet address):" -ForegroundColor Yellow
    Write-Host "      lattice-node --mine --coinbase YOUR_ADDRESS" -ForegroundColor Blue
    Write-Host ""
    Write-Host "📦 Releases & source:" -ForegroundColor Cyan
    Write-Host "   https://github.com/$GithubRepo/releases" -ForegroundColor Blue
    Write-Host ""
}

# ── Main ──────────────────────────────────────────────────────────────────────
Print-Header
Get-LatestRelease
Install-Binaries
Set-UserPath
New-DefaultConfig
New-Shortcuts
Print-Completion

