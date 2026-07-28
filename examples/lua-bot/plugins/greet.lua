description = "Greet the sender"

function on_command(ctx)
    local name = ctx.sender_name or ctx.sender_uuid
    ctx:reply("Hello, " .. name .. "! 👋")
end
