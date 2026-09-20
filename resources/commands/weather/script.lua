-- weather command using wttr.in API

local lang = jarvis.context.language

-- Use a city named in the command when present; otherwise use the saved city.
-- Examples: "погода в Казани" and "weather in London".
local phrase = jarvis.context.phrase or ""
local requested_city = phrase:match("погода%s+в%s+(.+)")
    or phrase:match("weather%s+in%s+(.+)")
local city = requested_city or jarvis.state.get("city") or "Moscow"

jarvis.log("info", "Fetching weather for: " .. city)

local function russian_number(value)
    local ones = {"ноль", "один", "два", "три", "четыре", "пять", "шесть", "семь", "восемь", "девять", "десять", "одиннадцать", "двенадцать", "тринадцать", "четырнадцать", "пятнадцать", "шестнадцать", "семнадцать", "восемнадцать", "девятнадцать"}
    local tens = { [20] = "двадцать", [30] = "тридцать", [40] = "сорок", [50] = "пятьдесят", [60] = "шестьдесят" }
    if value < 20 then return ones[value + 1] end
    local ten = math.floor(value / 10) * 10
    local rest = value % 10
    return tens[ten] .. (rest > 0 and " " .. ones[rest + 1] or "")
end

local encoded_city = city:gsub(" ", "%%20")
local weather = jarvis.http.json("https://wttr.in/" .. encoded_city .. "?format=j1&lang=" .. lang)

if weather and weather.current_condition and weather.current_condition[1] and weather.current_condition[1].temp_C then
    local temp = tonumber(weather.current_condition[1].temp_C)
    if temp then
        temp = temp >= 0 and math.floor(temp + 0.5) or math.ceil(temp - 0.5)
        local sign = temp > 0 and "+" or ""
        local title = lang == "ru" and "Погода" or "Weather"
        local visual = city .. ": " .. sign .. temp .. " °C"
        local spoken
        if lang == "ru" then
            local prefix = temp > 0 and "плюс " or (temp < 0 and "минус " or "")
            spoken = "Погода в городе " .. city .. ": " .. prefix .. russian_number(math.abs(temp)) .. " градусов Цельсия."
        else
            spoken = "Temperature in " .. city .. ": " .. sign .. temp .. " degrees Celsius."
        end
        jarvis.log("info", "Weather: " .. visual)
        jarvis.system.notify(title, visual)
        jarvis.speak(spoken)
    else
        jarvis.audio.play_error()
    end
else
    jarvis.log("error", "Failed to fetch weather for " .. city)
    jarvis.audio.play_error()
end

return { chain = false }
