#NoEnv
SendMode Input
SetWorkingDir %A_ScriptDir%

if WinExist("ahk_exe Discord.exe")
{
    WinActivate
    return
}

discord := A_LocalAppData "\\Discord\\Update.exe"
Run, %discord% --processStart Discord.exe
