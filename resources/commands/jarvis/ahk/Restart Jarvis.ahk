#NoEnv
SendMode Input
SetWorkingDir %A_ScriptDir%

; Wait for the current jarvis-app.exe process to finish before starting it again.
Sleep, 1500
target := A_ScriptDir "\\..\\..\\..\\..\\jarvis-app.exe"
Run, %target%
