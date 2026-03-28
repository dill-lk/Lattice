# Lattice Blockchain - One-Click Installer for Windows
# Downloads pre-built binaries from GitHub Releases

$ErrorActionPreference = "Stop"

# Configuration
$BIN_DIR = if ($env:LATTICE_BIN_DIR) { $env:LATTICE_BIN_DIR } else { "$env:USERPROFILE\.local\bin" }
$GITHUB_REPO = "dill-lk/Lattice"
$GITHUB_API  = "https://api.github.com/repos/$GITHUB_REPO/releases/latest"
$ASSET_NAME  = "lattice-windows-amd64.zip"

# Helper functions
function Print-Header {
    Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║                                                          ║" -ForegroundColor Cyan
    Write-Host "║          🚀 LATTICE BLOCKCHAIN INSTALLER 🚀             ║" -ForegroundColor Green
    Write-Host "║                                                          ║" -ForegroundColor Cyan
    Write-Host "║     Quantum-Resistant Blockchain with Advanced Features ║" -ForegroundColor Cyan
    Write-Host "║                                                          ║" -ForegroundColor Cyan
    Write-Host "╚══════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan
}

function Print-Step {
    param([string]$Message)
    Write-Host "==> " -ForegroundColor Blue -NoNewline
    Write-Host $Message -ForegroundColor Green
}

function Print-Info {
    param([string]$Message)
    Write-Host "i  " -ForegroundColor Cyan -NoNewline
    Write-Host $Message
}

function Print-Success {
    param([string]$Message)
    Write-Host "v  " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Print-Warning {
    param([string]$Message)
    Write-Host "!  " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Print-Error {
    param([string]$Message)
    Write-Host "x  " -ForegroundColor Red -NoNewline
    Write-Host $Message
}

# Fetch the latest release and return download URL + tag
function Get-LatestRelease {
    Print-Step "Fetching latest release from GitHub..."

    $release = Invoke-RestMethod -Uri $GITHUB_API -UseBasicParsing

    $script:ReleaseTag = $release.tag_name

    $asset = $release.assets | Where-Object { $_.name -eq $ASSET_NAME } | Select-Object -First 1

    if (-not $asset) {
        Print-Error "No pre-built binary found for $ASSET_NAME in release $script:ReleaseTag."
        Print-Info "See https://github.com/$GITHUB_REPO/releases for available assets."
        exit 1
    }

    $script:DownloadUrl = $asset.browser_download_url

    Print-Success "Latest release: $script:ReleaseTag"
}

# Download the zip and install binaries
function Install-Binaries {
    Print-Step "Downloading $ASSET_NAME..."

    $tmpDir  = [System.IO.Path]::GetTempPath() + [System.Guid]::NewGuid().ToString()
    New-Item -ItemType Directory -Path $tmpDir | Out-Null

    $archive = Join-Path $tmpDir $ASSET_NAME

    $ProgressPreference = 'SilentlyContinue'
    Invoke-WebRequest -Uri $script:DownloadUrl -OutFile $archive -UseBasicParsing
    $ProgressPreference = 'Continue'

    Print-Success "Download complete"
    Print-Step "Installing binaries to $BIN_DIR..."

    if (-not (Test-Path $BIN_DIR)) {
        New-Item -ItemType Directory -Path $BIN_DIR -Force | Out-Null
    }

    Expand-Archive -Path $archive -DestinationPath $tmpDir -Force

    $installed = 0
    foreach ($bin in @("lattice-node.exe", "lattice-cli.exe", "lattice-miner.exe")) {
        $src = Join-Path $tmpDir $bin
        if (Test-Path $src) {
            Copy-Item $src (Join-Path $BIN_DIR $bin) -Force
            $installed++
        }
    }

    Remove-Item -Recurse -Force $tmpDir

    if ($installed -eq 0) {
        Print-Error "No binaries were found in the archive. The release may be incomplete."
        exit 1
    }

    Print-Success "Installed $installed binaries to $BIN_DIR"
}

# Add BIN_DIR to the user PATH if not already present
function Setup-Path {
    Print-Step "Setting up PATH..."

    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")

    if ($currentPath -notlike "*$BIN_DIR*") {
        [Environment]::SetEnvironmentVariable("Path", "$currentPath;$BIN_DIR", "User")
        $env:PATH = "$env:PATH;$BIN_DIR"
        Print-Success "Added $BIN_DIR to PATH"
        Print-Warning "Restart your terminal for the PATH change to take effect"
    } else {
        Print-Success "PATH already configured"
    }
}

# Create default configuration
function Create-Config {
    Print-Step "Creating default configuration..."

    $configDir = "$env:USERPROFILE\.lattice\config"
    if (-not (Test-Path $configDir)) {
        New-Item -ItemType Directory -Path $configDir -Force | Out-Null
    }

    $configContent = @"
# Lattice Node Configuration

[network]
# P2P listen address
listen_addr = "/ip4/0.0.0.0/tcp/30333"
# Bootstrap nodes (empty for standalone)
bootstrap_nodes = []
# Maximum number of peers
max_peers = 50

[consensus]
# Mining threads (0 = auto-detect)
mining_threads = 0
# Mining difficulty (auto-adjust)
difficulty = 1000000

[rpc]
# RPC listen address
listen_addr = "127.0.0.1:8545"
# Enable RPC server
enabled = true

[storage]
# Database path
db_path = "$env:USERPROFILE\.lattice\data"
# Cache size in MB
cache_size = 256
"@

    $configContent | Out-File -FilePath "$configDir\node.toml" -Encoding utf8

    Print-Success "Configuration created at $configDir\node.toml"
}

# Create desktop shortcuts
function Create-Shortcuts {
    Print-Step "Creating desktop shortcuts..."

    $desktopPath = [Environment]::GetFolderPath("Desktop")
    $WshShell = New-Object -ComObject WScript.Shell

    # Node shortcut
    $shortcut = $WshShell.CreateShortcut("$desktopPath\Lattice Node.lnk")
    $shortcut.TargetPath = "$BIN_DIR\lattice-node.exe"
    $shortcut.WorkingDirectory = "$env:USERPROFILE\.lattice"
    $shortcut.Description = "Lattice Blockchain Node"
    $shortcut.Save()

    # CLI shortcut
    $shortcut = $WshShell.CreateShortcut("$desktopPath\Lattice CLI.lnk")
    $shortcut.TargetPath = "powershell.exe"
    $shortcut.Arguments = "-NoExit -Command `"cd '$env:USERPROFILE\.lattice'; Write-Host 'Lattice CLI Ready' -ForegroundColor Green`""
    $shortcut.WorkingDirectory = "$env:USERPROFILE\.lattice"
    $shortcut.Description = "Lattice Command Line"
    $shortcut.Save()

    Print-Success "Shortcuts created on Desktop"
}

# Print completion message
function Print-Completion {
    Write-Host ""
    Write-Host "╔══════════════════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "║                                                          ║" -ForegroundColor Green
    Write-Host "║        ✅ LATTICE INSTALLATION COMPLETE! ✅             ║" -ForegroundColor Green
    Write-Host "║                                                          ║" -ForegroundColor Green
    Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Green
    Write-Host ""
    Write-Host "📍 Installation Details:" -ForegroundColor Cyan
    Write-Host "   • Version:  $script:ReleaseTag" -ForegroundColor White
    Write-Host "   • Binaries: $BIN_DIR" -ForegroundColor White
    Write-Host "   • Config:   $env:USERPROFILE\.lattice\config" -ForegroundColor White
    Write-Host "   • Data:     $env:USERPROFILE\.lattice\data" -ForegroundColor White
    Write-Host ""
    Write-Host "🚀 Quick Start:" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "   1. Create a wallet:" -ForegroundColor Yellow
    Write-Host "      lattice-cli wallet create" -ForegroundColor Blue
    Write-Host ""
    Write-Host "   2. Start a node:" -ForegroundColor Yellow
    Write-Host "      lattice-node --config $env:USERPROFILE\.lattice\config\node.toml" -ForegroundColor Blue
    Write-Host ""
    Write-Host "   3. Check node status:" -ForegroundColor Yellow
    Write-Host "      lattice-cli node status" -ForegroundColor Blue
    Write-Host ""
    Write-Host "   4. Start mining:" -ForegroundColor Yellow
    Write-Host "      lattice-miner --threads 4" -ForegroundColor Blue
    Write-Host ""
    Write-Host "📚 Documentation:" -ForegroundColor Cyan
    Write-Host "   • GitHub: https://github.com/$GITHUB_REPO" -ForegroundColor White
    Write-Host ""
    Write-Host "💡 Need help?" -ForegroundColor Cyan
    Write-Host "   • GitHub: https://github.com/$GITHUB_REPO" -ForegroundColor White
    Write-Host ""
}

# Main installation flow
function Main {
    Print-Header
    Get-LatestRelease
    Install-Binaries
    Setup-Path
    Create-Config
    Create-Shortcuts
    Print-Completion
}

try {
    Main
} catch {
    Print-Error "Installation failed: $_"
    Write-Host $_.ScriptStackTrace
    exit 1
}

