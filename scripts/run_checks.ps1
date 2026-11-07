$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent $PSScriptRoot)

Write-Host "== Formatting check =="
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== Tests =="
cargo test --workspace
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== All-target check =="
cargo check --workspace --all-targets
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== Clippy =="
cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== Example =="
cargo run -p chronos-core --example basic
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== Release build =="
cargo build --workspace --release
exit $LASTEXITCODE
