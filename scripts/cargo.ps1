$projectRoot = Split-Path $PSScriptRoot -Parent
$localCargo = Join-Path $projectRoot '.tools\cargo\bin\cargo.exe'
if (Test-Path -LiteralPath $localCargo) {
    $env:CARGO_HOME = Join-Path $projectRoot '.tools\cargo'
    $env:RUSTUP_HOME = Join-Path $projectRoot '.tools\rustup'
    $env:PATH = (Join-Path $env:CARGO_HOME 'bin') + ';' + $env:PATH
    $cargoCommand = $localCargo
} else {
    $cargoCommand = (Get-Command cargo -ErrorAction Stop).Source
}
Push-Location $projectRoot
try { & $cargoCommand @args; $cargoResult = $LASTEXITCODE } finally { Pop-Location }
exit $cargoResult
