param([switch]$Upgraded, [string]$DependencyRoot = $env:CNU_VCPKG_INSTALLED, [string]$Generator = "Visual Studio 18 2026")
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$coreRoot = Join-Path $PSScriptRoot 'bitcoin-core'
$baseCommit = & git -C $coreRoot rev-parse HEAD
if ($baseCommit -ne 'e8e7e91a1144c378dff4da2e2a562eb0f3f2e1d6') { throw 'Unexpected Core base commit' }
if (!$Upgraded) {
    $changes = & git -C $coreRoot status --porcelain --untracked-files=normal
    if ($changes) { throw 'Stock build requires a clean upstream checkout. This checkout is already patched; use -Upgraded to rebuild the modified node.' }
} elseif (!(Test-Path -LiteralPath (Join-Path $coreRoot 'src\cnu_reserve\reserve.cpp'))) {
    throw 'Run compatibility/patch_core.py before an upgraded build.'
}
& (Join-Path $projectRoot 'scripts\cargo.ps1') build --locked --release -p core_bridge
if ($LASTEXITCODE -ne 0) { throw 'Rust bridge build failed' }
$vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
$vsRoot = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
$cmake = Join-Path $vsRoot 'Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe'
$toolchain = Join-Path $vsRoot 'VC\vcpkg\scripts\buildsystems\vcpkg.cmake'
if (!$DependencyRoot) { $DependencyRoot = Join-Path $env:USERPROFILE '.cache\nsb-regtest\installed' }
$vcpkgOptions = @('-DVCPKG_MANIFEST_NO_DEFAULT_FEATURES=ON')
if (Test-Path -LiteralPath $DependencyRoot) {
    $vcpkgOptions += @("-DVCPKG_INSTALLED_DIR=$DependencyRoot", '-DVCPKG_MANIFEST_INSTALL=OFF')
} else {
    $vcpkgOptions += '-DVCPKG_MANIFEST_INSTALL=ON'
}
& $cmake -S (Join-Path $projectRoot 'compatibility\bitcoin-core') -B (Join-Path $projectRoot 'compatibility\bitcoin-core\build') -G $Generator -A x64 `
    "-DCMAKE_TOOLCHAIN_FILE=$toolchain" @vcpkgOptions -DVCPKG_TARGET_TRIPLET=x64-windows-static `
    -DBUILD_GUI=OFF -DENABLE_WALLET=OFF -DBUILD_TESTS=OFF -DBUILD_BENCH=OFF -DBUILD_TX=OFF -DBUILD_UTIL=OFF -DBUILD_BITCOIN_BIN=OFF -DWITH_ZMQ=OFF `
    "-DCNU_RUST_LIBRARY=$projectRoot/target/release/core_bridge.lib"
if ($LASTEXITCODE -ne 0) { throw 'CMake configuration failed' }
& $cmake --build (Join-Path $projectRoot 'compatibility\bitcoin-core\build') --config Release --target bitcoind bitcoin-cli -j 8
if ($LASTEXITCODE -ne 0) { throw 'Core build failed' }

$flavor = if ($Upgraded) { 'upgraded' } else { 'stock' }
$destination = Join-Path $PSScriptRoot "bin\$flavor"
New-Item -ItemType Directory -Force -Path $destination | Out-Null
Copy-Item -LiteralPath (Join-Path $projectRoot 'compatibility\bitcoin-core\build\bin\Release\bitcoind.exe') -Destination $destination
