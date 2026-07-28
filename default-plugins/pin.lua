description = "Pin a message. Use {::prefix}pin <text> to pin, or {::prefix}pins to view/remove pinned messages."
aliases = {"pins"}

function on_command(ctx)
    if ctx.trigger == "pins" then
        if ctx.args[1] == "rm" and ctx.args[2] then
            local idx = tonumber(ctx.args[2])
            if not idx then
                ctx:reply("Please provide a valid pin number to remove. Example: {::prefix}pins rm 1")
                return
            end
            
            local contents = ctx:read_file("pinned.txt")
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
            ctx:write_file("pinned.txt", table.concat(lines, "\n") .. (table.getn(lines) > 0 and "\n" or ""))
            ctx:reply("✅ Pin " .. idx .. " removed.")
            return
        end

        local contents = ctx:read_file("pinned.txt")
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
    ctx:append_file("pinned.txt", text .. "\n")
    ctx:reply("✅ Pinned!")
end
