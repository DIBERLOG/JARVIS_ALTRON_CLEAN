if not jarvis.system.open("tg://") then
    error("Не удалось открыть Telegram")
end
return { chain = false }
