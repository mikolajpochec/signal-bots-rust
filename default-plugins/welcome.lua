description = "Generate a warm welcome message for someone. Format: {::prefix}welcome <name>"
group = "AI"

function on_command(ctx)
    if #ctx.args == 0 then
        ctx:reply("Who should I welcome? Usage: {::prefix}welcome <name>")
        return
    end
    
    local name = table.concat(ctx.args, " ")
    local msg_ts = ctx:reply_get_timestamp("Rolling out the red carpet for " .. name .. "... 🎈")
    
    local prompt = "Write a very warm, slightly funny, and overly enthusiastic welcome message for a person named " .. name .. " who just joined the group chat. Make them feel incredibly special."
    
    local status, response = pcall(function() return ctx:llm_generate(prompt) end)
    if status and response and response ~= "" then
        ctx:edit_message(msg_ts, response)
    else
        print("AI generation error: " .. tostring(response))
        ctx:edit_message(msg_ts, "❌ Failed to generate a welcome message, but welcome anyway, " .. name .. "!")
    end
end
