# Unified quick-start mining script for Lattice (Windows)

$ErrorActionPreference = "Stop"
$DefaultWallet = "wallet.json"

Write-Host "`n╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║        ⛏️  LATTICE QUICK START MINING (UNIFIED)         ║" -ForegroundColor Green
Write-Host "╚══════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

if (-not (Get-Command lattice -ErrorAction SilentlyContinue)) {
    Write-Host "Lattice not found. Install with:" -ForegroundColor Yellow
    Write-Host "  irm https://raw.githubusercontent.com/dill-lk/Lattice/main/install.ps1 | iex" -ForegroundColor White
    exit 1
}

Write-Host "✓ unified lattice CLI found`n" -ForegroundColor Green

if (-not (Test-Path $DefaultWallet)) {
    Write-Host "No local default wallet found. Creating one...`n" -ForegroundColor Yellow
    lattice --wallet-new
}

Write-Host "✓ wallet ready`n" -ForegroundColor Green

try {
    $walletJson = lattice --json wallet address --wallet $DefaultWallet | Out-String | ConvertFrom-Json
    $walletAddr = $walletJson.address
    if ($walletAddr) {
        Write-Host "Mining to: " -ForegroundColor Cyan -NoNewline
        Write-Host "$walletAddr`n" -ForegroundColor White
    }
} catch {
    $walletAddr = $null
}

$cores = (Get-CimInstance Win32_Processor | Measure-Object -Property NumberOfLogicalProcessors -Sum).Sum
if (-not $cores) { $cores = 4 }
$threads = [Math]::Max(1, [Math]::Floor($cores * 0.75))

Write-Host "CPU Cores detected: " -ForegroundColor Cyan -NoNewline
Write-Host $cores -ForegroundColor White
Write-Host "Mining threads: " -ForegroundColor Cyan -NoNewline
Write-Host "$threads`n" -ForegroundColor White

Write-Host "Starting unified local mining..." -ForegroundColor Green
Write-Host "This path will auto-start local integrated miner-node mode if needed.`n" -ForegroundColor Cyan

lattice --mine $threads
