description = "Show bot uptime"

-- Capture load time
local start_time = os.time()

function on_command(ctx)
    local elapsed = os.time() - start_time
    local hours = math.floor(elapsed / 3600)
    local mins = math.floor((elapsed % 3600) / 60)
    local secs = elapsed % 60
    ctx:reply(string.format("⏱ Uptime: %02d:%02d:%02d", hours, mins, secs))
end
