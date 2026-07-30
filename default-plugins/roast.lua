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

    local formatted_history = "Here is chat history:\n"
    for i = #history, 1, -1 do
        formatted_history = formatted_history .. history[i] .. "\n"
    end
    
    local prompt = formatted_history .. "\nRoast the person named " .. target .. " based on these recent messages from the chat. Be brutal but funny."
    
    local msg_ts = ctx:reply_get_timestamp("Thinking... 🤔")
    
    local status, response = pcall(function() return ctx:llm_generate(prompt) end)
    if status and response and response ~= "" then
        ctx:edit_message(msg_ts, response)
    else
        print("AI generation error: " .. tostring(response))
        ctx:edit_message(msg_ts, "❌ I couldn't come up with a good roast.")
    end
end
