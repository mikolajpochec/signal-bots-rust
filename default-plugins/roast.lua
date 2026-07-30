description = "Roast a user based on their recent messages. Usage: {::prefix}roast <name>"

function on_command(ctx)
    local target = table.concat(ctx.args, " ")
    if target == "" then
        ctx:reply("Who should I roast? Usage: {::prefix}roast <name>")
        return
    end

    local history = ctx:get_chat_history(15)
    if not history or #history == 0 then
        ctx:reply("I don't have enough message history in this chat to roast anyone yet!")
        return
    end

    local prompt = "Roast the person named " .. target .. " based on these recent messages from the chat. Be brutal but funny."
    
    local status, response = pcall(function() return ctx:llm_generate_with_context(prompt, history) end)
    if status and response and response ~= "" then
        ctx:reply(response)
    else
        ctx:reply("❌ Failed to generate roast. Error: " .. tostring(response))
    end
end
