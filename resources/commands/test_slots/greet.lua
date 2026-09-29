local phrase = jarvis.context.phrase or ""
local addressed_person = phrase:match("^поздоровайся с%s+(.+)$")
local name = phrase:match("^привет%s+(.+)$") or jarvis.context.slots.name

if addressed_person then
    jarvis.speak("Приветствую. Рад знакомству.")
elseif not name or name == "" then
    jarvis.speak("Назовите имя, и я поздороваюсь.")
else
    jarvis.speak("Здравствуйте, " .. name .. ".")
end

return { chain = false }
