param(
  [int]$Port = 7777
)

$env:RUST_LOG = "info"
Write-Host "Starting core-daemon on port $Port ..."
cd "$PSScriptRoot\..\crates\core-daemon"
cargo run
