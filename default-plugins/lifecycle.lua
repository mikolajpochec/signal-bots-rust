description = "System lifecycle plugin (not invoked by users)"

function on_spawn(ctx)
    local date_str = os.date("%Y-%m-%d %H:%M:%S")
    ctx:broadcast("🟢 **Bot Started**\nSystem online at: " .. date_str)
end

function on_death(ctx)
    local date_str = os.date("%Y-%m-%d %H:%M:%S")
    local uptime = ctx.bot_uptime
    local hours = math.floor(uptime / 3600)
    local mins = math.floor((uptime % 3600) / 60)
    local secs = uptime % 60
    
    local uptime_str = ""
    if hours > 0 then
        uptime_str = uptime_str .. hours .. "h "
    end
    if mins > 0 or hours > 0 then
        uptime_str = uptime_str .. mins .. "m "
    end
    uptime_str = uptime_str .. secs .. "s"

    ctx:broadcast("🔴 **Bot Shutting Down**\nTime: " .. date_str .. "\nUptime: " .. uptime_str)
end
