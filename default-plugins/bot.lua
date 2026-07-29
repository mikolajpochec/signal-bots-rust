description = "Ask the bot a question."

function on_command(ctx)
    if #ctx.args == 0 then
        ctx:reply("Please ask a question! Usage: {::prefix}bot <question>")
        return
    end
    
    local prompt = table.concat(ctx.args, " ")
    local status, response = pcall(function() return ctx:llm_generate(prompt) end)
    if status and response then
        ctx:reply(response)
    else
        ctx:reply("❌ Failed to generate response.")
    end
end
