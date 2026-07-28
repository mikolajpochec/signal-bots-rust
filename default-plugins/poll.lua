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
        ctx:append_file("polls.txt", tostring(timestamp) .. ":" .. question .. "\n")
        for i = 1, #options do
            ctx:react_to(timestamp, emojis[i])
        end
    end
end

function on_reaction(ctx)
    if not ctx.reaction_target_timestamp then return end
    
    local target_ts = tostring(ctx.reaction_target_timestamp)
    
    local polls_content = ctx:read_file("polls.txt")
    if not polls_content or polls_content == "" then return end
    
    local is_poll = false
    local poll_question = ""
    
    for line in string.gmatch(polls_content, "[^\r\n]+") do
        local ts, q = string.match(line, "^(%d+):(.*)$")
        if ts == target_ts then
            is_poll = true
            poll_question = q
            break
        end
    end
    
    if is_poll then
        -- Heuristic: Ignore reactions that happen within 5 seconds of the poll creation (these are the bot's own buttons)
        if not ctx.reaction_is_remove and (ctx.timestamp - ctx.reaction_target_timestamp < 5000) then
            return
        end

        local user = ctx.sender_name or ctx.sender_number or "Someone"
        local action = "voted for"
        if ctx.reaction_is_remove then
            action = "withdrew their vote of"
        end
        
        ctx:reply(user .. " " .. action .. " " .. ctx.reaction_emoji .. " on poll: " .. poll_question)
    end
end
