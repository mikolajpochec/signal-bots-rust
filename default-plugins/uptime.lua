description = "Show bot uptime"

function on_command(ctx)
    local elapsed = ctx.bot_uptime
    local hours = math.floor(elapsed / 3600)
    local mins = math.floor((elapsed % 3600) / 60)
    local secs = elapsed % 60
    ctx:reply(string.format("⏱ Uptime: %02d:%02d:%02d", hours, mins, secs))
end
