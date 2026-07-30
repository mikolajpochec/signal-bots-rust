description = "Check the vibe of the chat. Format: {::prefix}vibe"
group = "AI"

function on_command(ctx)
    local msg_ts = ctx:reply_get_timestamp("Checking vibes... 🔮")
    
    local history = ctx:get_chat_history(20) or {}
    local formatted_history = ""
    for i = #history, 1, -1 do
        formatted_history = formatted_history .. history[i] .. "\n"
    end
    
    if formatted_history == "" then
        ctx:edit_message(msg_ts, "The vibe is empty. Say something!")
        return
    end
    
    local prompt = "Here is the recent chat history:\n" .. formatted_history .. "\nAnalyze the vibe of this conversation and give it a funny 'vibe rating' with percentages (e.g. 75% chaotic, 25% wholesome) along with a short explanation."
    
    local status, response = pcall(function() return ctx:llm_generate(prompt) end)
    if status and response and response ~= "" then
        ctx:edit_message(msg_ts, response)
    else
        print("AI generation error: " .. tostring(response))
        ctx:edit_message(msg_ts, "❌ The vibes were too powerful to compute.")
    end
end
