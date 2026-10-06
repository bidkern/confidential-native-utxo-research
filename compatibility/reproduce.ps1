param([string]$DependencyRoot = $env:CNU_VCPKG_INSTALLED, [string]$Generator = 'Visual Studio 18 2026')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$coreRoot = Join-Path $PSScriptRoot 'bitcoin-core'
$pinnedCommit = 'e8e7e91a1144c378dff4da2e2a562eb0f3f2e1d6'
if (Test-Path -LiteralPath $coreRoot) { throw 'Use a fresh extracted source package. Refusing to overwrite an existing Core checkout.' }
Push-Location $projectRoot
try {
    & git init $coreRoot
    if ($LASTEXITCODE) { throw 'git init failed' }
    & git -C $coreRoot remote add origin https://github.com/bitcoin/bitcoin.git
    & git -C $coreRoot fetch --depth=1 origin $pinnedCommit
    if ($LASTEXITCODE) { throw 'Pinned upstream fetch failed' }
    & git -C $coreRoot checkout --detach FETCH_HEAD
    if ($LASTEXITCODE -or (& git -C $coreRoot rev-parse HEAD) -ne $pinnedCommit) { throw 'Wrong upstream revision' }
    & ./scripts/cargo.ps1 test --locked --workspace
    if ($LASTEXITCODE) { throw 'Rust tests failed' }
    & ./compatibility/build.ps1 -DependencyRoot $DependencyRoot -Generator $Generator
    if ($LASTEXITCODE) { throw 'Stock build failed' }
    & python compatibility/patch_core.py
    if ($LASTEXITCODE) { throw 'Source patch failed' }
    & ./compatibility/build.ps1 -Upgraded -DependencyRoot $DependencyRoot -Generator $Generator
    if ($LASTEXITCODE) { throw 'Upgraded build failed' }
    foreach ($demo in @('v2_demo.py','activation_demo.py','hostile_demo.py')) {
        & python (Join-Path $PSScriptRoot $demo)
        if ($LASTEXITCODE) { throw "Failed: $demo" }
    }
} finally { Pop-Location }
