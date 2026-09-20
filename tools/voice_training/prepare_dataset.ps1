param(
    [string]$Source = "$env:USERPROFILE\Downloads",
    [string]$Output = "$PSScriptRoot\dataset",
    [string]$Ffmpeg = ""
)

$python = Get-Command py -ErrorAction SilentlyContinue
if (-not $python) {
    throw "Python Launcher (py.exe) is required. Install Python 3.10 first."
}

if (-not $Ffmpeg) {
    $Ffmpeg = Get-Command ffmpeg -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -First 1
}

if (-not $Ffmpeg) {
    $packageRoot = Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Packages\Gyan.FFmpeg_Microsoft.Winget.Source_8wekyb3d8bbwe'
    $Ffmpeg = Get-ChildItem -Path $packageRoot -Filter ffmpeg.exe -Recurse -ErrorAction SilentlyContinue |
        Select-Object -First 1 -ExpandProperty FullName
}

if (-not $Ffmpeg) {
    throw "ffmpeg.exe is required. Install FFmpeg and reopen PowerShell, or pass -Ffmpeg <path>."
}

& py -3.10 "$PSScriptRoot\prepare_dataset.py" --source $Source --output $Output --ffmpeg $Ffmpeg
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
