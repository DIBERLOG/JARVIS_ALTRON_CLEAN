$ErrorActionPreference = "Stop"

$python = Join-Path $PSScriptRoot ".venv\Scripts\python.exe"
if (-not (Test-Path $python)) {
    throw "Virtual environment not found. Create it with: py -3.10 -m venv .venv"
}

& $python -m pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu128
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

& $python -c "import torch; assert torch.cuda.is_available(), 'CUDA is not available'; print(torch.__version__); print(torch.cuda.get_device_name(0))"
