#NoEnv
SendMode Input
SetWorkingDir %A_ScriptDir%
SetTitleMatchMode, 2

if WinExist("ahk_exe Discord.exe")
{
    WinRestore
    WinActivate
    ExitApp
}

discord := A_LocalAppData . "\Discord\Discord.exe"
if FileExist(discord)
{
    Run, % """" discord """",, UseErrorLevel
}
else
{
    updater := A_LocalAppData . "\Discord\Update.exe"
    if !FileExist(updater)
        ExitApp, 1
    Run, % """" updater """ --processStart Discord.exe",, UseErrorLevel
}

if ErrorLevel
    ExitApp, 1
