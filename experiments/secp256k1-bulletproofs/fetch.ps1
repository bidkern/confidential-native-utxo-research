$ErrorActionPreference = 'Stop'
$pin = 'b247e1ec8ed62b9abf123dc83189d253e17d488d'
$source = Join-Path $PSScriptRoot 'upstream'
if (-not (Test-Path -LiteralPath $source)) {
    git clone --no-checkout https://github.com/mimblewimble/secp256k1-zkp.git $source
    if ($LASTEXITCODE -ne 0) { throw 'Upstream clone failed' }
    git -C $source checkout --detach $pin
    if ($LASTEXITCODE -ne 0) { throw 'Pinned checkout failed' }
}
$actual = git -C $source rev-parse HEAD
if ($LASTEXITCODE -ne 0 -or $actual -ne $pin) { throw 'Unexpected upstream commit' }
$dirty = git -C $source status --porcelain
if ($LASTEXITCODE -ne 0 -or $dirty) { throw 'Upstream checkout must be clean' }
$env:CNU_BP_SOURCE = $source
Write-Host "Pinned historical source ready; CNU_BP_SOURCE=$source"
