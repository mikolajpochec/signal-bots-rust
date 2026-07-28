description = "Set a reminder. Format: /remind 5m Take out trash"

function on_command(ctx)
    if not ctx.args or #ctx.args == 0 then
        ctx:reply("Error: Unrecognized format or missing message.")
        return
    end

    local text = table.concat(ctx.args, " ")
    local delay = nil
    local msg = nil

    -- 1. Date + Time (YYYY-MM-DD HH:MM)
    local y, mon, d, H, M, rest = string.match(text, "^(%d%d%d%d)%-(%d%d)%-(%d%d)%s+(%d%d):(%d%d)%s+(.+)")
    if y then
        delay = os.time{year=tonumber(y), month=tonumber(mon), day=tonumber(d), hour=tonumber(H), min=tonumber(M), sec=0} - os.time()
        msg = rest
    end

    -- 2. Date only (YYYY-MM-DD)
    if not delay then
        local y2, mon2, d2, rest2 = string.match(text, "^(%d%d%d%d)%-(%d%d)%-(%d%d)%s+(.+)")
        if y2 then
            delay = os.time{year=tonumber(y2), month=tonumber(mon2), day=tonumber(d2), hour=17, min=0, sec=0} - os.time()
            msg = rest2
        end
    end

    -- 3. Hours
    if not delay then
        local h, rest3 = string.match(text, "^(%d+)%s*hours%s+(.+)")
        if not h then h, rest3 = string.match(text, "^(%d+)%s*hour%s+(.+)") end
        if not h then h, rest3 = string.match(text, "^(%d+)%s*h%s+(.+)") end
        if h then
            delay = tonumber(h) * 3600
            msg = rest3
        end
    end

    -- 4. Minutes
    if not delay then
        local min, rest4 = string.match(text, "^(%d+)%s*minutes%s+(.+)")
        if not min then min, rest4 = string.match(text, "^(%d+)%s*minute%s+(.+)") end
        if not min then min, rest4 = string.match(text, "^(%d+)%s*m%s+(.+)") end
        if min then
            delay = tonumber(min) * 60
            msg = rest4
        end
    end

    if not delay or not msg then
        ctx:reply("Error: Unrecognized format or missing message.")
        return
    end

    if delay <= 0 then
        ctx:reply("Error: That time is in the past!")
        return
    end

    ctx:schedule_reply(delay, "⏰ Reminder: " .. msg)
    ctx:reply("Reminder set for " .. tostring(delay) .. " seconds from now!")
end
