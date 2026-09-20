$ErrorActionPreference = "Stop"
$env:PYTHONUTF8 = "1"

& "$PSScriptRoot\.venv\Scripts\python.exe" "$PSScriptRoot\run_smoke_train.py"
exit $LASTEXITCODE
