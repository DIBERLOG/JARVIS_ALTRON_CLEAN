$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
$restoreScript = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../resources/commands/windows/windows.ps1')).Path
$restoreTestDir = Join-Path ([IO.Path]::GetTempPath()) ('jarvis-window-test-' + [Guid]::NewGuid().ToString('N'))
$null = [IO.Directory]::CreateDirectory($restoreTestDir)
$restoreState = Join-Path $restoreTestDir 'state.json'
$normalForm = [Windows.Forms.Form]::new()
$normalForm.Text = 'JARVIS restore regression: normal'
$normalForm.Width=300; $normalForm.Height=180
$maximizedForm = [Windows.Forms.Form]::new()
$maximizedForm.Text = 'JARVIS restore regression: maximized'
$previousForm = [Windows.Forms.Form]::new()
$previousForm.Text = 'JARVIS restore regression: previously minimized'

function Invoke-TestWindowAction([string]$action) {
    # -File does not parse a comma-separated array, so use the single test process's handles via a wrapper.
    $testCode = "& '$($restoreScript.Replace("'","''"))' -Action $action -StatePath '$($restoreState.Replace("'","''"))' -WindowHandles @($($normalForm.Handle.ToInt64()),$($maximizedForm.Handle.ToInt64()),$($previousForm.Handle.ToInt64()))"
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($testCode))
    $testErrorPath=Join-Path $restoreTestDir 'helper-error.log'
    $testProcess = Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-NonInteractive','-WindowStyle','Hidden','-EncodedCommand',$encoded) -WindowStyle Hidden -PassThru -RedirectStandardError $testErrorPath
    $testDeadline=[DateTime]::UtcNow.AddSeconds(20)
    while (!$testProcess.HasExited) {
        [Windows.Forms.Application]::DoEvents()
        Start-Sleep -Milliseconds 20
        if ([DateTime]::UtcNow -gt $testDeadline) { throw 'Window test helper timed out.' }
    }
    if ($testProcess.ExitCode -ne 0) { Get-Content -LiteralPath $testErrorPath; throw ('Window helper failed: '+$testProcess.ExitCode) }
    for ($index=0;$index -lt 15;$index++) { [Windows.Forms.Application]::DoEvents();Start-Sleep -Milliseconds 20 }
}

try {
    $normalForm.Show(); $maximizedForm.Show(); $previousForm.Show()
    $maximizedForm.WindowState=[Windows.Forms.FormWindowState]::Maximized
    $previousForm.WindowState=[Windows.Forms.FormWindowState]::Minimized
    [Windows.Forms.Application]::DoEvents()
    Invoke-TestWindowAction 'Minimize'
    if ($normalForm.WindowState -ne 'Minimized' -or $maximizedForm.WindowState -ne 'Minimized') { throw 'Test windows were not minimized.' }
    Invoke-TestWindowAction 'Minimize'
    $saved=Get-Content -LiteralPath $restoreState -Raw | ConvertFrom-Json
    if (@($saved.windows).Count -ne 2) { throw 'Repeated minimize lost the original snapshot or captured previously minimized windows.' }
    Invoke-TestWindowAction 'Restore'
    if ($normalForm.WindowState -ne 'Normal') { throw 'Normal window was not restored.' }
    if ($maximizedForm.WindowState -ne 'Maximized') { throw 'Maximized window lost its previous state.' }
    if ($previousForm.WindowState -ne 'Minimized') { throw 'Previously minimized window was incorrectly restored.' }
    $saved=Get-Content -LiteralPath $restoreState -Raw | ConvertFrom-Json
    if (@($saved.windows).Count -ne 0) { throw 'Restored windows were not cleared from the snapshot.' }
    Write-Output 'PASS: repeated minimize, normal/maximized restore and previously minimized exclusion (test windows only)'
} finally {
    $normalForm.Dispose();$maximizedForm.Dispose();$previousForm.Dispose()
}
