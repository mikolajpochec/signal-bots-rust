description = "Ask the bot a question."

function on_command(ctx)
    if #ctx.args == 0 then
        ctx:reply("Please ask a question! Usage: {::prefix}bot <question>")
        return
    end
    
    local prompt = table.concat(ctx.args, " ")
    
    -- Fetch the last 10 messages from the chat to give the bot context!
    local history = ctx:get_chat_history(10) or {}
    
    local status, response = pcall(function() 
        return ctx:llm_generate_with_context(prompt, history) 
    end)
    
    if status and response and response ~= "" then
        ctx:reply(response)
    else
        ctx:reply("❌ Failed to generate response. Error: " .. tostring(response))
    end
end
