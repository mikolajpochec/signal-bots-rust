description = "Summarize the last 50 messages. Format: {::prefix}tldr"
group = "AI"

function on_command(ctx)
    local msg_ts = ctx:reply_get_timestamp("Reading history... 📚")
    
    local history = ctx:get_chat_history(50) or {}
    local formatted_history = ""
    for i = #history, 1, -1 do
        formatted_history = formatted_history .. history[i] .. "\n"
    end
    
    if formatted_history == "" then
        ctx:edit_message(msg_ts, "There is no history to summarize yet!")
        return
    end
    
    local prompt = "Here is the recent chat history:\n" .. formatted_history .. "\nSummarize the key points of what was discussed in a few concise bullet points. Be funny and informal."
    
    local status, response = pcall(function() return ctx:llm_generate(prompt) end)
    if status and response and response ~= "" then
        ctx:edit_message(msg_ts, response)
    else
        print("AI generation error: " .. tostring(response))
        ctx:edit_message(msg_ts, "❌ I encountered an error while summarizing.")
    end
end
