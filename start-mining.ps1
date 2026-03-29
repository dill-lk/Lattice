# Quick start mining script for Lattice Blockchain (Windows)

$ErrorActionPreference = "Stop"

Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║                                                                              ║" -ForegroundColor Cyan
Write-Host "║        ⛏️  LATTICE QUICK START MINING ⛏️                                    ║" -ForegroundColor Green
Write-Host "║                                                                              ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

# Check if lattice is installed
if (-not (Get-Command lattice-node -ErrorAction SilentlyContinue)) {
    Write-Host "Lattice not found. Install with:" -ForegroundColor Yellow
    Write-Host "  irm https://latticechain.io/install.ps1 | iex" -ForegroundColor White
    exit 1
}

Write-Host "✓ Lattice is installed`n" -ForegroundColor Green

# Check for wallet
if (-not (Test-Path "wallet.json") -and -not (Test-Path "$env:USERPROFILE\.lattice\wallet.json")) {
    Write-Host "No wallet found. Creating one...`n" -ForegroundColor Yellow
    lattice-cli wallet create
    Write-Host ""
}

Write-Host "✓ Wallet ready`n" -ForegroundColor Green

# Get wallet address
try {
    $walletAddr = (lattice-cli wallet address 2>$null | Select-String "lat1").Matches.Value
    if ($walletAddr) {
        Write-Host "Mining to: " -ForegroundColor Cyan -NoNewline
        Write-Host "$walletAddr`n" -ForegroundColor White
    }
} catch {
    $walletAddr = $null
}

# Detect CPU cores
$cores = (Get-WmiObject -Class Win32_Processor).NumberOfLogicalProcessors
$threads = [Math]::Max(1, [Math]::Floor($cores * 0.75))

Write-Host "CPU Cores detected: " -ForegroundColor Cyan -NoNewline
Write-Host $cores -ForegroundColor White
Write-Host "Mining threads: " -ForegroundColor Cyan -NoNewline
Write-Host "$threads`n" -ForegroundColor White

# Check if node is running
try {
    $response = Invoke-WebRequest -Uri "http://localhost:8545/health" -Method GET -TimeoutSec 2 -ErrorAction Stop
    Write-Host "✓ Node is running`n" -ForegroundColor Green
} catch {
    Write-Host "Node not running. Starting in dev mode...`n" -ForegroundColor Yellow
    Start-Process -FilePath "lattice-node" -ArgumentList "--dev" -WindowStyle Hidden
    Start-Sleep -Seconds 5
    Write-Host "✓ Node started`n" -ForegroundColor Green
}

# Start mining
Write-Host "Starting miner with $threads threads...`n" -ForegroundColor Green
Write-Host "╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║  Press Ctrl+C to stop mining                                               ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

if ($walletAddr) {
    lattice-miner --threads $threads --address $walletAddr
} else {
    lattice-miner --threads $threads
}
