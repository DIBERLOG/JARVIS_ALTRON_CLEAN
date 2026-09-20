#NoEnv
SetTitleMatchMode, 2

if WinExist("Twitch")
{
    WinActivate
    Send, ^w
}
