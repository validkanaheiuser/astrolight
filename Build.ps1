$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot 'src-tauri')
try {
  cargo build --locked --release --bin astrolight
  if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
  $releaseDirectory = Join-Path $PSScriptRoot 'dist'
  New-Item -ItemType Directory -Force -Path $releaseDirectory | Out-Null
  Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'src-tauri\target\release\astrolight.exe') -Destination (Join-Path $releaseDirectory 'Astrolight.exe')
  Write-Host "Built: $releaseDirectory\Astrolight.exe"
} finally { Pop-Location }
