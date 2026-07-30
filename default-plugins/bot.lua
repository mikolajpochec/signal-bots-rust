description = "Ask the bot a question."

function on_command(ctx)
    if #ctx.args == 0 then
        ctx:reply("Please ask a question! Usage: {::prefix}bot <question>")
        return
    end
    
    local prompt = table.concat(ctx.args, " ")
    
    local history = ctx:get_chat_history(10) or {}
    local formatted_history = "Here is chat history:\n"
    for i = #history, 1, -1 do
        formatted_history = formatted_history .. history[i] .. "\n"
    end
    
    local current_sender = ctx.sender_name or "Unknown"
    local final_prompt = formatted_history .. "\nHere is current message:\n" .. current_sender .. ": " .. prompt
    
    local status, response = pcall(function() 
        return ctx:llm_generate(final_prompt) 
    end)
    
    if status and response and response ~= "" then
        ctx:reply(response)
    else
        ctx:reply("❌ Failed to generate response. Error: " .. tostring(response))
    end
end
