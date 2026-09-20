#NoEnv
SendMode Input
SetWorkingDir %A_ScriptDir%

if WinExist("ahk_exe Code.exe")
{
    WinActivate
    return
}

code := A_LocalAppData "\\Programs\\Microsoft VS Code\\Code.exe"
if FileExist(code)
    Run, "%code%"
else
    Run, code
