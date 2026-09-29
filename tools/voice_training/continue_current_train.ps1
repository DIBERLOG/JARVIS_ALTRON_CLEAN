$ErrorActionPreference = 'Stop'
$env:PYTHONUTF8 = '1'
$env:JARVIS_TRAIN_EPOCHS = '3'
$env:JARVIS_TRAIN_RUN = 'jarvis_ru_current_continue'
$env:JARVIS_TRAIN_RESTORE = Join-Path $PSScriptRoot 'smoke_output\jarvis_ru_current-September-28-2026_11+29PM-e6007eb\best_model.pth'
& (Join-Path $PSScriptRoot '.venv\Scripts\python.exe') (Join-Path $PSScriptRoot 'run_smoke_train.py')
exit $LASTEXITCODE
