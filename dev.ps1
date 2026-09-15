# dev.ps1 - Start Peerino in dev mode
# Workaround for Windows Defender race condition with cargo run.
# Usage: .\dev.ps1

Write-Host "Starting Peerino dev environment..." -ForegroundColor Cyan
Write-Host ""

# Step 1: Start Vite dev server in background (use cmd.exe because npm is not a Win32 executable)
Write-Host "1/4 Starting Vite dev server..." -ForegroundColor Yellow
$vite = Start-Process -NoNewWindow -PassThru -FilePath "cmd.exe" -ArgumentList "/c", "npm run dev" -WorkingDirectory (Get-Location).Path

# Step 2: Wait for Vite to be ready on port 5173
Write-Host "2/4 Waiting for Vite (5s)..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# Step 3: Build the Rust backend
Write-Host "3/4 Building Tauri backend..." -ForegroundColor Yellow
cargo build --manifest-path src-tauri/Cargo.toml

# Step 4: Wait for Defender scan, then launch the executable
Write-Host "4/4 Waiting for Defender scan (3s)..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

Write-Host "Launching Peerino..." -ForegroundColor Green
Start-Process -FilePath "src-tauri\target\debug\Peerino.exe"

Write-Host ""
Write-Host "Peerino is running. Close this window or press Ctrl+C to stop." -ForegroundColor Cyan
Write-Host "To stop Vite: Stop-Process -Id $($vite.Id)" -ForegroundColor DarkGray