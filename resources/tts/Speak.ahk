#NoEnv
SendMode Input
SetWorkingDir %A_ScriptDir%

text := ""
for index, argument in A_Args
{
    if (index > 1)
        text .= " "
    text .= argument
}

if (text != "")
{
    speaker := ComObjCreate("SAPI.SpVoice")
    voices := speaker.GetVoices()
    Loop % voices.Count
    {
        candidate := voices.Item(A_Index - 1)
        if InStr(candidate.GetDescription(), "Irina")
        {
            speaker.Voice := candidate
            break
        }
    }
    speaker.Rate := -1
    speaker.Speak(text)
}
