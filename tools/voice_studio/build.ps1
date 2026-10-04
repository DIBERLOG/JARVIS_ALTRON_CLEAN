$ErrorActionPreference = 'Stop'
$studioDir = $PSScriptRoot
$studioPython = Join-Path $studioDir '.venv/Scripts/python.exe'
if (!(Test-Path -LiteralPath $studioPython)) { throw 'Установите отдельную среду интерфейса перед сборкой.' }
Push-Location $studioDir
try {
    & $studioPython -m PyInstaller --noconfirm --clean --onefile --windowed --name 'JARVIS Voice Studio' --add-data 'index.html;.' --add-data 'train_runner.py;.' --add-data 'manrope-latin.woff2;.' --add-data 'manrope-cyrillic.woff2;.' --add-data 'Manrope-LICENSE.txt;.' --collect-all webview --hidden-import pystray._win32 studio.py
    if ($LASTEXITCODE -ne 0) { throw 'Сборка не завершилась.' }
} finally { Pop-Location }
