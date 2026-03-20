#!/usr/bin/env pwsh
# Photonix Camera — Device Benchmark Runner (Windows PowerShell)
# Runs criterion benchmarks on a connected Android device via ADB
# Usage: .\scripts\run_device_bench.ps1

param(
    [string]$DeviceId = "",
    [string]$OutputFile = "benchmark_results.md"
)

$ADB = "adb"
if ($DeviceId) { $ADB = "adb -s $DeviceId" }

Write-Host "Photonix Camera — Device Benchmark Runner"
Write-Host "=========================================`n"

# Check device connected
$devices = & adb devices | Select-String "device$"
if (-not $devices) {
    Write-Error "No Android device connected. Run 'adb devices' to check."
    exit 1
}

Write-Host "Connected device: $devices"

# Build bench binary for Android
Write-Host "`nBuilding benchmark binary for ARM64..."
Set-Location rust
cargo ndk -t arm64-v8a build --release --benches 2>&1 | Select-String "Compiling|Finished|error"

# Find the bench binary
$benchBinary = Get-ChildItem "target\aarch64-linux-android\release\deps\pipeline_bench*" |
    Where-Object { -not $_.Name.EndsWith(".d") } |
    Select-Object -First 1

if (-not $benchBinary) {
    Write-Error "Benchmark binary not found. Build may have failed."
    Set-Location ..
    exit 1
}

Write-Host "Binary: $($benchBinary.Name)"

# Push to device
Write-Host "`nPushing binary to device..."
& adb push $benchBinary.FullName /data/local/tmp/pipeline_bench
& adb shell chmod +x /data/local/tmp/pipeline_bench

# Run benchmarks
Write-Host "`nRunning benchmarks on device (this takes ~5 minutes)..."
$results = & adb shell /data/local/tmp/pipeline_bench --bench 2>&1

Set-Location ..

# Parse and format results
Write-Host "`nResults:"
Write-Host "========`n"

$tableRows = @()
foreach ($line in $results) {
    if ($line -match "(\S+)\s+time:\s+\[([0-9.]+\s+\w+)\s+([0-9.]+\s+\w+)\s+([0-9.]+\s+\w+)\]") {
        $name = $Matches[1]
        $p50  = $Matches[2]
        $p95  = $Matches[3]
        $tableRows += "| $name | $p50 | $p95 |"
    }
}

# Write markdown table
$mdContent = @"
# Photonix Camera — Benchmark Results
Generated: $(Get-Date -Format "yyyy-MM-dd HH:mm")

## Benchmark baseline table

| Stage              | Target  | Actual p50 | Actual p95 | Pass? |
|--------------------|---------|------------|------------|-------|
| Scene classify     | <10ms   |            |            |       |
| Burst align 3fr    | <120ms  |            |            |       |
| DnCNN denoise      | <40ms   |            |            |       |
| MiDaS depth        | <80ms   |            |            |       |
| Real-ESRGAN tile   | <60ms   |            |            |       |
| Full portrait      | <330ms  |            |            |       |
| Full night         | <260ms  |            |            |       |
| Full landscape     | <300ms  |            |            |       |

## Raw criterion output

| Benchmark | p50 | p95 |
|-----------|-----|-----|
$($tableRows -join "`n")
"@

$mdContent | Out-File -FilePath $OutputFile -Encoding utf8
Write-Host "Results written to: $OutputFile"
Write-Host $mdContent