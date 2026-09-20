#NoEnv  ; Recommended for performance and compatibility with future AutoHotkey releases.
; #Warn  ; Enable warnings to assist with detecting common errors.
SendMode Input  ; Recommended for new scripts due to its superior speed and reliability.
SetWorkingDir %A_ScriptDir%  ; Ensures a consistent starting directory.

; Reuse an existing browser window when one is already open.
; This keeps the command from creating a second browser window every time.
SetTitleMatchMode, 2
GroupAdd, browsers, ahk_class MozillaWindowClass
GroupAdd, browsers, ahk_exe msedge.exe
GroupAdd, browsers, ahk_exe chrome.exe
GroupAdd, browsers, ahk_exe firefox.exe
GroupAdd, browsers, ahk_exe browser.exe ; Yandex Browser
GroupAdd, browsers, ahk_exe opera.exe
GroupAdd, browsers, ahk_exe brave.exe

if WinExist("ahk_group browsers")
{
    WinActivate
    return
}

RegRead, BrowserKeyName, HKEY_CURRENT_USER, Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.html\UserChoice, Progid
RegRead, BrowserFullCommand, HKEY_CLASSES_ROOT, %BrowserKeyName%\shell\open\command
StringGetPos, pos, BrowserFullCommand, ",,1
pos := --pos
StringMid, BrowserPathandEXE, BrowserFullCommand, 2, %pos%
Run, % BrowserPathandEXE
