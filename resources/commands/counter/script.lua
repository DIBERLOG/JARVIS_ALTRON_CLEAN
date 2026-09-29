-- simple counter demonstrating state persistence

local count = jarvis.state.get("count") or 0
count = count + 1
jarvis.state.set("count", count)

local lang = jarvis.context.language
local msg = lang == "ru"
    and "Счётчик: " .. count
    or "Counter: " .. count

jarvis.log("info", msg)
local title = lang == "ru" and "Счётчик JARVIS" or "JARVIS counter"
local primary = lang == "ru" and ("Выполнено: " .. count) or ("Completed: " .. count)
local detail = lang == "ru" and "Команд обработано" or "Commands processed"
jarvis.system.notify(title, primary .. "\n" .. detail)
jarvis.speak(msg)

return { chain = false }
