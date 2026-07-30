description = "Pin a message. Use {::prefix}pin <text> to pin. Use {::prefix}pins to view pins."
aliases = { pins = "View pinned messages.", ["pin-rm"] = "Remove a pinned message. Usage: {::prefix}pin-rm <id>" }
group = "Tools"

local function get_filename(ctx)
    if ctx.is_group then
        return "pinned_" .. ctx.group_id .. ".txt"
    else
        return "pinned_" .. (ctx.sender_uuid or ctx.sender_number) .. ".txt"
    end
end

function on_command(ctx)
    local filename = get_filename(ctx)
    if ctx.trigger == "pin-rm" then
        if not ctx.args[1] then
            ctx:reply("Please provide a valid pin number to remove. Example: {::prefix}pin-rm 1")
            return
        end
        local idx = tonumber(ctx.args[1])
        if not idx then
            ctx:reply("Please provide a valid pin number to remove. Example: {::prefix}pin-rm 1")
            return
        end
        
        local contents = ctx:read_file(filename)
        if not contents or contents == "" then
            ctx:reply("No pinned messages to remove.")
            return
        end
        
        local lines = {}
        for line in string.gmatch(contents, "[^\r\n]+") do
            table.insert(lines, line)
        end
        
        if idx < 1 or idx > #lines then
            ctx:reply("Pin number out of bounds. There are only " .. #lines .. " pins.")
            return
        end
        
        table.remove(lines, idx)
        ctx:write_file(filename, table.concat(lines, "\n") .. (#lines > 0 and "\n" or ""))
        ctx:reply("✅ Pin " .. idx .. " removed.")
        return
    end

    if ctx.trigger == "pins" then
        local contents = ctx:read_file(filename)
        if not contents or contents == "" then
            ctx:reply("No pinned messages.")
        else
            local lines = {}
            for line in string.gmatch(contents, "[^\r\n]+") do
                table.insert(lines, line)
            end
            
            local msg = "📌 **Pinned Messages:**\n"
            for i, line in ipairs(lines) do
                msg = msg .. i .. ". " .. line .. "\n"
            end
            ctx:reply(msg)
        end
        return
    end

    if not ctx.args or #ctx.args == 0 then
        ctx:reply("Please provide text to pin. Example: {::prefix}pin Hello world")
        return
    end
    
    local text = table.concat(ctx.args, " ")
    -- Remove bullet if they added one manually
    text = string.gsub(text, "^%-%s*", "")
    ctx:append_file(filename, text .. "\n")
    ctx:reply("✅ Pinned!")
end
