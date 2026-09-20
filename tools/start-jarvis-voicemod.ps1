# Starts the assistant only after Voicemod has had time to initialize its audio devices.
# This script is invoked by the per-user "Jarvis + Voicemod" logon task.

$ErrorActionPreference = 'Stop'

$voicemod = 'C:\Program Files\Voicemod V3\Voicemod.exe'
$jarvisRoot = 'C:\Users\angel\Documents\ChatGPT\Jarvis_X_Altron_clean_staging'
$jarvisApp = Join-Path $jarvisRoot 'target\debug\jarvis-app.exe'
$jarvisGui = Join-Path $jarvisRoot 'target\debug\jarvis-gui.exe'

function Start-IfNotRunning {
    param(
        [Parameter(Mandatory = $true)][string]$ProcessName,
        [Parameter(Mandatory = $true)][string]$FilePath,
        [string]$WorkingDirectory
    )

    if (-not (Get-Process -Name $ProcessName -ErrorAction SilentlyContinue)) {
        $startParams = @{ FilePath = $FilePath }
        if ($WorkingDirectory) {
            $startParams.WorkingDirectory = $WorkingDirectory
        }
        Start-Process @startParams
    }
}

if (-not (Test-Path -LiteralPath $voicemod)) { throw "Voicemod not found: $voicemod" }
if (-not (Test-Path -LiteralPath $jarvisApp)) { throw "Jarvis engine not found: $jarvisApp" }
if (-not (Test-Path -LiteralPath $jarvisGui)) { throw "Jarvis interface not found: $jarvisGui" }

Start-IfNotRunning -ProcessName 'Voicemod' -FilePath $voicemod

# The virtual devices are available after logon, but Voicemod still needs a few seconds
# to restore its selected voice and audio routing.
Start-Sleep -Seconds 10

Start-IfNotRunning -ProcessName 'jarvis-app' -FilePath $jarvisApp -WorkingDirectory $jarvisRoot
Start-Sleep -Seconds 2
Start-IfNotRunning -ProcessName 'jarvis-gui' -FilePath $jarvisGui -WorkingDirectory $jarvisRoot
