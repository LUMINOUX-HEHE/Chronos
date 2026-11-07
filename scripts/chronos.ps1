$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$binary = Join-Path $repoRoot "target\release\chronos.exe"

$buildInputs = @(
    (Join-Path $repoRoot "Cargo.toml"),
    (Join-Path $repoRoot "Cargo.lock"),
    (Join-Path $repoRoot "rust-toolchain.toml"),
    (Join-Path $repoRoot "crates\chronos-core\Cargo.toml")
)

$sourceRoot = Join-Path $repoRoot "crates\chronos-core\src"
if (Test-Path $sourceRoot) {
    $buildInputs += Get-ChildItem -Path $sourceRoot -Filter "*.rs" -File -Recurse |
        Select-Object -ExpandProperty FullName
}

$needsBuild = -not (Test-Path $binary)
if (-not $needsBuild) {
    $binaryTime = (Get-Item $binary).LastWriteTimeUtc
    foreach ($inputPath in $buildInputs) {
        if ((Test-Path $inputPath) -and (Get-Item $inputPath).LastWriteTimeUtc -gt $binaryTime) {
            $needsBuild = $true
            break
        }
    }
}

if ($needsBuild) {
    Push-Location $repoRoot
    try {
        Write-Host "Building the current CHRONOS release executable..."
        cargo build --workspace --release
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
    finally {
        Pop-Location
    }
}

& $binary @args
exit $LASTEXITCODE
