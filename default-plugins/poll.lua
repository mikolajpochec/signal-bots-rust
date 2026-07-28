description = "Create a poll. Format: /poll Question | Option 1 | Option 2"

function on_command(ctx)
    if not ctx.args or #ctx.args == 0 then
        ctx:reply("Error: Please provide a question and at least one option. Format: /poll Question | Option 1 | Option 2")
        return
    end

    local text = table.concat(ctx.args, " ")
    
    local parts = {}
    for part in string.gmatch(text, "([^|]+)") do
        local trimmed = string.match(part, "^%s*(.-)%s*$")
        if trimmed ~= "" then
            table.insert(parts, trimmed)
        end
    end
    
    if #parts < 2 then
        ctx:reply("Error: Please provide a question and at least one option. Format: /poll Question | Option 1 | Option 2")
        return
    end
    
    local question = parts[1]
    
    local emojis = {"1️⃣", "2️⃣", "3️⃣", "4️⃣", "5️⃣", "6️⃣", "7️⃣", "8️⃣", "9️⃣"}
    
    local options = {}
    for i = 2, #parts do
        if i - 1 <= #emojis then
            table.insert(options, parts[i])
        end
    end
    
    local message_parts = {question, ""}
    for i, opt in ipairs(options) do
        table.insert(message_parts, emojis[i] .. " " .. opt)
    end
    
    local message = table.concat(message_parts, "\n")
    
    local timestamp = ctx:reply_get_timestamp(message)
    
    if timestamp then
        for i = 1, #options do
            ctx:react_to(timestamp, emojis[i])
        end
    end
end
