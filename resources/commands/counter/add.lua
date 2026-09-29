local count = (tonumber(jarvis.state.get("count")) or 0) + 1
jarvis.state.set("count", count)
local ru = jarvis.context.language == "ru"
jarvis.system.notify(ru and "Счётчик JARVIS" or "JARVIS counter", (ru and "Плюс один · теперь " or "Plus one · now ") .. count)
jarvis.speak((ru and "Теперь на счётчике " or "Counter is now ") .. count)
return { chain = false }
