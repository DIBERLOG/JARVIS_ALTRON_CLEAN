#NoEnv
SetTitleMatchMode, 2

if WinExist("ahk_exe Discord.exe")
    WinClose

Process, Exist, Discord.exe
if ErrorLevel
    RunWait, %ComSpec% /C taskkill /T /F /IM Discord.exe,, Hide
