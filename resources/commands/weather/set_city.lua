-- set city for weather command

local phrase = jarvis.context.phrase or ""
local lang = jarvis.context.language

local city = jarvis.context.slots and jarvis.context.slots.city
    or phrase:match("город%s+(.+)") or phrase:match("city%s+(.+)")

if city and type(city) == "string" then
    city = city:gsub("^%s*(.-)%s*$", "%1") -- trim
    city = city:gsub("^на%s+", "")
    city = city:gsub("^to%s+", "")
    if city == "" then jarvis.audio.play_not_found(); return { chain = false } end
    
    -- save to state (shared with weather command)
    jarvis.state.set("city", city)
    
    local msg = lang == "ru" 
        and "Город установлен: " .. city
        or "City set to: " .. city
    
    jarvis.log("info", msg)
    jarvis.system.notify("Jarvis", msg)
    jarvis.speak(msg)
else
    local msg = lang == "ru"
        and "Не удалось определить город"
        or "Could not determine city"
    
    jarvis.log("warn", msg)
    jarvis.audio.play_not_found()
end

return { chain = false }
