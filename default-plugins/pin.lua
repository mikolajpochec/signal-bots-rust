description = "Pin a message. Use {::prefix}pin <text> to pin, or {::prefix}pins to view pinned messages."

function on_command(ctx)
    if not ctx.args or #ctx.args == 0 then
        local contents = ctx:read_file("pinned.txt")
        if not contents or contents == "" then
            ctx:reply("No pinned messages.")
        else
            ctx:reply(contents)
        end
    else
        local text = table.concat(ctx.args, " ")
        ctx:append_file("pinned.txt", "- " .. text .. "\n")
        ctx:reply("Pinned!")
    end
end
