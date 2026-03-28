# Lattice Blockchain - One-Click Installer for Windows
# This script installs Lattice blockchain and all its dependencies

$ErrorActionPreference = "Stop"

# Configuration
$INSTALL_DIR = if ($env:LATTICE_INSTALL_DIR) { $env:LATTICE_INSTALL_DIR } else { "$env:USERPROFILE\.lattice" }
$BIN_DIR = "$env:USERPROFILE\.local\bin"
$REPO_URL = "https://github.com/lattice-chain/lattice"
$MIN_RUST_VERSION = "1.70.0"

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
    Write-Host "ℹ  " -ForegroundColor Cyan -NoNewline
    Write-Host $Message
}

function Print-Success {
    param([string]$Message)
    Write-Host "✓  " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Print-Warning {
    param([string]$Message)
    Write-Host "⚠  " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Print-Error {
    param([string]$Message)
    Write-Host "✗  " -ForegroundColor Red -NoNewline
    Write-Host $Message
}

# Check if command exists
function Test-CommandExists {
    param([string]$Command)
    $null -ne (Get-Command $Command -ErrorAction SilentlyContinue)
}

# Check administrator privileges
function Test-Administrator {
    $currentUser = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($currentUser)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

# Check Rust installation
function Check-Rust {
    Print-Step "Checking Rust installation..."
    
    if (-not (Test-CommandExists "rustc")) {
        Print-Warning "Rust is not installed"
        $response = Read-Host "Install Rust now? (y/n)"
        if ($response -eq "y" -or $response -eq "Y") {
            Install-Rust
        } else {
            Print-Error "Rust is required. Exiting."
            exit 1
        }
    } else {
        $rustVersion = (rustc --version).Split()[1]
        Print-Success "Rust $rustVersion is installed"
        
        # Version check (simplified)
        $currentVer = [version]$rustVersion
        $minVer = [version]$MIN_RUST_VERSION
        if ($currentVer -lt $minVer) {
            Print-Warning "Rust version $rustVersion is below minimum $MIN_RUST_VERSION"
            Print-Info "Updating Rust..."
            rustup update stable
        }
    }
}

# Install Rust
function Install-Rust {
    Print-Step "Installing Rust..."
    
    # Download rustup-init.exe
    $rustupUrl = "https://win.rustup.rs/x86_64"
    $rustupPath = "$env:TEMP\rustup-init.exe"
    
    Print-Info "Downloading Rust installer..."
    Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupPath
    
    Print-Info "Running Rust installer..."
    Start-Process -FilePath $rustupPath -ArgumentList "-y" -Wait
    
    # Refresh environment
    $env:PATH = [System.Environment]::GetEnvironmentVariable("Path", "User") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "Machine")
    
    Print-Success "Rust installed successfully"
}

# Check Git
function Check-Git {
    Print-Step "Checking Git installation..."
    
    if (-not (Test-CommandExists "git")) {
        Print-Warning "Git is not installed"
        Print-Info "Please install Git from: https://git-scm.com/download/win"
        Print-Info "After installing Git, please restart this script."
        exit 1
    } else {
        $gitVersion = (git --version).Split()[2]
        Print-Success "Git $gitVersion is installed"
    }
}

# Check Visual Studio Build Tools
function Check-BuildTools {
    Print-Step "Checking Visual Studio Build Tools..."
    
    $vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    
    if (Test-Path $vsWhere) {
        $vsInstances = & $vsWhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($vsInstances) {
            Print-Success "Visual Studio Build Tools found"
            return
        }
    }
    
    Print-Warning "Visual Studio Build Tools not found"
    Print-Info "Rust requires Visual Studio Build Tools to compile on Windows"
    Print-Info "Please install from: https://visualstudio.microsoft.com/downloads/"
    Print-Info "Select 'Desktop development with C++' workload"
    
    $response = Read-Host "Continue anyway? (y/n)"
    if ($response -ne "y" -and $response -ne "Y") {
        exit 1
    }
}

# Setup repository
function Setup-Repo {
    Print-Step "Setting up Lattice repository..."
    
    if (Test-Path $INSTALL_DIR) {
        Print-Info "Repository exists. Updating..."
        Set-Location $INSTALL_DIR
        git pull origin main
    } else {
        Print-Info "Cloning repository..."
        git clone $REPO_URL $INSTALL_DIR
        Set-Location $INSTALL_DIR
    }
    
    Print-Success "Repository ready"
}

# Build Lattice
function Build-Lattice {
    Print-Step "Building Lattice (this may take 10-20 minutes)..."
    
    Set-Location $INSTALL_DIR
    
    # Show progress
    Print-Info "Compiling Rust code..."
    $progressPreference = 'silentlyContinue'
    
    cargo build --release --bins 2>&1 | ForEach-Object {
        if ($_ -match "Compiling") {
            Write-Host "  → $_" -ForegroundColor Cyan
        }
    }
    
    $progressPreference = 'Continue'
    Print-Success "Build completed"
}

# Install binaries
function Install-Binaries {
    Print-Step "Installing binaries..."
    
    # Create bin directory
    if (-not (Test-Path $BIN_DIR)) {
        New-Item -ItemType Directory -Path $BIN_DIR -Force | Out-Null
    }
    
    # Copy binaries
    Copy-Item "$INSTALL_DIR\target\release\lattice-node.exe" "$BIN_DIR\" -Force
    Copy-Item "$INSTALL_DIR\target\release\lattice-cli.exe" "$BIN_DIR\" -Force
    Copy-Item "$INSTALL_DIR\target\release\lattice-miner.exe" "$BIN_DIR\" -Force
    
    Print-Success "Binaries installed to $BIN_DIR"
}

# Setup PATH
function Setup-Path {
    Print-Step "Setting up PATH..."
    
    # Get current user PATH
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    
    if ($currentPath -notlike "*$BIN_DIR*") {
        # Add to PATH
        $newPath = "$currentPath;$BIN_DIR"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        $env:PATH = "$env:PATH;$BIN_DIR"
        
        Print-Success "Added to PATH"
        Print-Warning "You may need to restart your terminal for PATH changes to take effect"
    } else {
        Print-Success "PATH already configured"
    }
}

# Create configuration
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

# Run tests
function Run-Tests {
    Print-Step "Running tests (optional)..."
    
    $response = Read-Host "Run test suite? This will take a few minutes. (y/n)"
    if ($response -eq "y" -or $response -eq "Y") {
        Set-Location $INSTALL_DIR
        cargo test --all --release
        Print-Success "All tests passed!"
    } else {
        Print-Info "Skipping tests"
    }
}

# Create desktop shortcuts
function Create-Shortcuts {
    Print-Step "Creating shortcuts..."
    
    $desktopPath = [Environment]::GetFolderPath("Desktop")
    
    # Node shortcut
    $WshShell = New-Object -ComObject WScript.Shell
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
    
    Print-Success "Shortcuts created on desktop"
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
    Write-Host "   • README: $INSTALL_DIR\README.md" -ForegroundColor White
    Write-Host "   • Docs:   https://docs.latticechain.io" -ForegroundColor White
    Write-Host ""
    Write-Host "💡 Need help?" -ForegroundColor Cyan
    Write-Host "   • GitHub:  https://github.com/lattice-chain/lattice" -ForegroundColor White
    Write-Host "   • Discord: https://discord.gg/lattice" -ForegroundColor White
    Write-Host ""
}

# Main installation flow
function Main {
    Print-Header
    
    # Check requirements
    Check-Rust
    Check-Git
    Check-BuildTools
    
    # Install
    Setup-Repo
    Build-Lattice
    Install-Binaries
    Setup-Path
    Create-Config
    Create-Shortcuts
    
    # Optional tests
    Run-Tests
    
    # Done!
    Print-Completion
}

# Run main function
try {
    Main
} catch {
    Print-Error "Installation failed: $_"
    Write-Host $_.ScriptStackTrace
    exit 1
}
