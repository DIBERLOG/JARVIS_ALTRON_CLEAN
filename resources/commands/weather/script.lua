-- weather command using wttr.in API

local lang = jarvis.context.language

-- Use a city named in the command when present; otherwise use the saved city.
-- Examples: "погода в Казани" and "weather in London".
local phrase = jarvis.context.phrase or ""
local requested_city = phrase:match("погода%s+в%s+(.+)")
    or phrase:match("weather%s+in%s+(.+)")
local city = requested_city or jarvis.state.get("city") or "Moscow"

jarvis.log("info", "Fetching weather for: " .. city)

-- build URL
local url = "https://wttr.in/" .. city .. "?format=3&lang=" .. lang

-- make request
local response = jarvis.http.get(url)

if response.ok then
    jarvis.log("info", "Weather: " .. response.body)
    
    -- show notification
    local title = lang == "ru" and "Погода" or "Weather"
    jarvis.system.notify(title, response.body)
    jarvis.speak(response.body)
else
    jarvis.log("error", "Failed to fetch weather: " .. (response.error or "unknown error"))
    jarvis.audio.play_error()
end

return { chain = false }
