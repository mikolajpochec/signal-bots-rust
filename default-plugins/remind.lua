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

    -- 3. Relative time (seconds, minutes, hours, days, weeks)
    if not delay then
        local amt, unit, rest3 = string.match(text, "^(%d+)%s*([a-zA-Z]+)%s*(.*)")
        if amt and unit then
            amt = tonumber(amt)
            unit = string.lower(unit)
            
            if unit == "s" or unit == "sec" or unit == "secs" or unit == "second" or unit == "seconds" then
                delay = amt
            elseif unit == "m" or unit == "min" or unit == "mins" or unit == "minute" or unit == "minutes" then
                delay = amt * 60
            elseif unit == "h" or unit == "hr" or unit == "hrs" or unit == "hour" or unit == "hours" then
                delay = amt * 3600
            elseif unit == "d" or unit == "day" or unit == "days" then
                delay = amt * 86400
            elseif unit == "w" or unit == "week" or unit == "weeks" then
                delay = amt * 604800
            end
            
            if delay then
                msg = rest3
                if msg == "" then
                    msg = "Time is up!"
                end
            end
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
