# Optional engine for stems on Windows: Demucs on PyTorch (CPU), in its own environment outside
# the app. Needs uv (winget install astral-sh.uv); uv downloads its own Python, so none is needed.
# Run it from your own terminal, not from an agent app packaged as MSIX, whose AppData writes
# Windows redirects into the package:
#   powershell -ExecutionPolicy Bypass -File scripts\setup-separation.ps1
# Optional arguments: the Python version, such as -Python 3.11, and -Target for another folder
# (curado finds the default one by itself; any other is chosen in Configuracion > Audio).
param(
    [string]$Python = '3.12',
    [string]$Target = (Join-Path $env:APPDATA 'studio.curado.player\separation')
)
$ErrorActionPreference = 'Stop'

$uv = Get-Command uv -ErrorAction SilentlyContinue
if (-not $uv) {
    Write-Error 'uv is missing. Install it with: winget install --id astral-sh.uv -e   (then open a new terminal)'
}
$repo = Split-Path -Parent $PSScriptRoot
$interpreter = Join-Path $Target 'Scripts\python.exe'

if (-not (Test-Path $interpreter)) {
    & uv venv --python $Python $Target
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
& uv pip install --python $interpreter -r (Join-Path $repo 'scripts\requirements-separation-windows.txt')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host 'Listo. Abre curado: Configuracion > Audio > Stems. El modelo (unos 80 MB) se descarga al primer uso.'
