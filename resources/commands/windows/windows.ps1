param(
    [Parameter(Mandatory=$true)][ValidateSet('Minimize','Restore','Inspect')][string]$Action,
    [string]$StatePath = (Join-Path $env:LOCALAPPDATA 'JarvisAltron/windows/minimized-v1.json'),
    [long[]]$WindowHandles = @()
)
$ErrorActionPreference = 'Stop'

if (-not ('JarvisWindows' -as [type])) {
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
public class JarvisWindowInfo {
    public long Handle;
    public uint ProcessId;
    public long Started;
    public bool Maximized;
}
public static class JarvisWindows {
    private delegate bool EnumProc(IntPtr hwnd, IntPtr parameter);
    [DllImport("user32.dll")] private static extern bool EnumWindows(EnumProc callback, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
    [DllImport("user32.dll")] private static extern bool IsZoomed(IntPtr hwnd);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint processId);
    [DllImport("user32.dll")] private static extern int GetWindowLong(IntPtr hwnd, int index);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] private static extern int GetWindowTextLength(IntPtr hwnd);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] private static extern int GetClassName(IntPtr hwnd, StringBuilder name, int size);
    [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr hwnd, int command);
    [DllImport("dwmapi.dll")] private static extern int DwmGetWindowAttribute(IntPtr hwnd, int attribute, out int value, int size);
    public static JarvisWindowInfo[] Capture() {
        var windows = new List<JarvisWindowInfo>();
        EnumWindows(delegate(IntPtr hwnd, IntPtr unused) {
            if (!IsWindowVisible(hwnd) || IsIconic(hwnd) || GetWindowTextLength(hwnd)==0) return true;
            if ((GetWindowLong(hwnd, -20) & 0x80)!=0) return true; // tool windows
            int cloaked;
            if (DwmGetWindowAttribute(hwnd, 14, out cloaked, 4)==0 && cloaked!=0) return true;
            var name = new StringBuilder(256); GetClassName(hwnd, name, name.Capacity);
            if (name.ToString()=="Progman" || name.ToString()=="WorkerW" || name.ToString().StartsWith("Shell_")) return true;
            uint processId; GetWindowThreadProcessId(hwnd, out processId);
            if (processId == Process.GetCurrentProcess().Id) return true;
            try {
                using(var process = Process.GetProcessById((int)processId)) {
                    windows.Add(new JarvisWindowInfo { Handle=hwnd.ToInt64(), ProcessId=processId,
                        Started=process.StartTime.ToUniversalTime().Ticks, Maximized=IsZoomed(hwnd) });
                }
            } catch { } // inaccessible/protected processes are left untouched
            return true;
        }, IntPtr.Zero);
        return windows.ToArray();
    }
    public static bool Matches(long handle, uint expectedId, long started) {
        IntPtr hwnd=new IntPtr(handle); if (!IsWindow(hwnd)) return false;
        uint processId; GetWindowThreadProcessId(hwnd, out processId);
        if (processId != expectedId) return false;
        try {
            using(var process=Process.GetProcessById((int)processId)) {
                return process.StartTime.ToUniversalTime().Ticks == started;
            }
        } catch { return false; }
    }
}
'@
}

if ($Action -eq 'Inspect') {
    @([JarvisWindows]::Capture() | Where-Object { !$WindowHandles.Count -or $_.Handle -in $WindowHandles })
    return
}

$windowsMutex = [Threading.Mutex]::new($false, 'Local\JarvisAltronWindowCommandsV1')
$windowsLock = $false
try {
    try { $windowsLock = $windowsMutex.WaitOne(10000) } catch [Threading.AbandonedMutexException] { $windowsLock = $true }
    if (!$windowsLock) { throw 'Another window command is still running.' }
    $saved = @()
    if (Test-Path -LiteralPath $StatePath) {
        $state = Get-Content -LiteralPath $StatePath -Raw | ConvertFrom-Json
        if ($state.version -ne 1) { throw 'Unsupported window snapshot.' }
        $saved = @($state.windows | Where-Object { [JarvisWindows]::Matches($_.Handle,$_.ProcessId,$_.Started) })
    }
    if ($Action -eq 'Minimize') {
        $captured = @([JarvisWindows]::Capture() | Where-Object { !$WindowHandles.Count -or $_.Handle -in $WindowHandles })
        # A repeated minimize must not replace the original snapshot with an empty list.
        foreach ($item in $captured) {
            if (!($saved | Where-Object { $_.Handle -eq $item.Handle })) { $saved += $item }
        }
    } else {
        foreach ($item in $saved) {
            if ($WindowHandles.Count -and $item.Handle -notin $WindowHandles) { continue }
            if ([JarvisWindows]::IsIconic([IntPtr]$item.Handle)) {
                $mode = if ($item.Maximized) { 3 } else { 9 }
                $null = [JarvisWindows]::ShowWindowAsync([IntPtr]$item.Handle, $mode)
            }
        }
        # Retain only windows that have not completed their restore yet.
        Start-Sleep -Milliseconds 300
        $saved = @($saved | Where-Object { [JarvisWindows]::Matches($_.Handle,$_.ProcessId,$_.Started) -and [JarvisWindows]::IsIconic([IntPtr]$_.Handle) })
    }
    $stateDirectory = [IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($StatePath))
    $null = [IO.Directory]::CreateDirectory($stateDirectory)
    $temporary = $StatePath + '.tmp'
    [IO.File]::WriteAllText($temporary, (@{version=1;windows=@($saved)} | ConvertTo-Json -Depth 4), [Text.UTF8Encoding]::new($false))
    if ([IO.File]::Exists($StatePath)) { [IO.File]::Replace($temporary,$StatePath,($StatePath+'.bak')) }
    else { [IO.File]::Move($temporary,$StatePath) }
    if ($Action -eq 'Minimize') {
        foreach ($item in $captured) { $null = [JarvisWindows]::ShowWindowAsync([IntPtr]$item.Handle, 6) }
    }
} finally {
    if ($windowsLock) { $windowsMutex.ReleaseMutex() }
    $windowsMutex.Dispose()
}
