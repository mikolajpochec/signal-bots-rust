description = "Create a poll. Format: {::prefix}poll Question | Option 1 | 👍=Option 2"
group = "Tools"

function trim(s)
    return s:match("^%s*(.-)%s*$")
end

function on_command(ctx)
    if not ctx.args or #ctx.args == 0 then
        ctx:reply("Error: Please provide a question and at least one option. Format: {::prefix}poll Question | Option 1 | 👍=Option 2")
        return
    end

    local text = table.concat(ctx.args, " ")
    local parts = {}
    for part in string.gmatch(text, "([^|]+)") do
        local trimmed = trim(part)
        if trimmed ~= "" then
            table.insert(parts, trimmed)
        end
    end
    
    if #parts < 2 then
        ctx:reply("Error: Please provide a question and at least one option.")
        return
    end
    
    local question = parts[1]
    local default_emojis = {"1️⃣", "2️⃣", "3️⃣", "4️⃣", "5️⃣", "6️⃣", "7️⃣", "8️⃣", "9️⃣", "🔟"}
    
    local options = {}
    local default_idx = 1
    
    for i = 2, #parts do
        local opt = parts[i]
        local emoji, opt_text = string.match(opt, "^([^=]+)=(.+)$")
        if emoji and opt_text then
            table.insert(options, {emoji = trim(emoji), text = trim(opt_text), count = 0})
        else
            if default_idx <= #default_emojis then
                table.insert(options, {emoji = default_emojis[default_idx], text = opt, count = 0})
                default_idx = default_idx + 1
            else
                table.insert(options, {emoji = "❓", text = opt, count = 0})
            end
        end
    end
    
    local message_parts = {question, ""}
    for i, opt in ipairs(options) do
        table.insert(message_parts, opt.emoji .. " " .. opt.text .. " (0)")
    end
    
    local message = table.concat(message_parts, "\n")
    local timestamp = ctx:reply_get_timestamp(message)
    
    if timestamp then
        -- Save state to file
        local lines = {question}
        for i, opt in ipairs(options) do
            table.insert(lines, opt.emoji .. "|" .. tostring(opt.count) .. "|" .. opt.text)
        end
        ctx:write_file("poll_" .. tostring(timestamp) .. ".txt", table.concat(lines, "\n"))
        
        -- Add reactions
        for i, opt in ipairs(options) do
            ctx:react_to(timestamp, opt.emoji)
        end
    end
end

function on_reaction(ctx)
    if not ctx.reaction_target_timestamp then return end
    local ts_str = tostring(ctx.reaction_target_timestamp)
    local filename = "poll_" .. ts_str .. ".txt"
    
    local content = ctx:read_file(filename)
    if not content or content == "" then return end
    
    local is_alias = false
    -- Resolve alias if the user reacted to an edited message
    if string.sub(content, 1, 6) == "ALIAS:" then
        is_alias = true
        filename = string.sub(content, 7)
        content = ctx:read_file(filename)
        if not content or content == "" then return end
    end
    
    -- Heuristic: Ignore reactions that happen within 5 seconds of the poll creation (bot's own buttons).
    -- We skip this for aliases because aliases mean the message has already been edited (poll is >5s old).
    if not is_alias and not ctx.reaction_is_remove and (ctx.timestamp - ctx.reaction_target_timestamp < 5000) then
        return
    end
    
    local lines = {}
    for line in string.gmatch(content, "[^\r\n]+") do
        table.insert(lines, line)
    end
    
    if #lines < 2 then return end
    
    local question = lines[1]
    local options = {}
    local changed = false
    
    for i = 2, #lines do
        local e, c, t = string.match(lines[i], "^([^|]+)|(%d+)|(.*)$")
        if e and c and t then
            local count = tonumber(c)
            if e == ctx.reaction_emoji then
                if ctx.reaction_is_remove then
                    count = math.max(0, count - 1)
                else
                    count = count + 1
                end
                changed = true
            end
            table.insert(options, {emoji = e, count = count, text = t})
        end
    end
    
    if changed then
        -- Save new state
        local new_lines = {question}
        local message_parts = {question, ""}
        for i, opt in ipairs(options) do
            table.insert(new_lines, opt.emoji .. "|" .. tostring(opt.count) .. "|" .. opt.text)
            table.insert(message_parts, opt.emoji .. " " .. opt.text .. " (" .. tostring(opt.count) .. ")")
        end
        
        ctx:write_file(filename, table.concat(new_lines, "\n"))
        
        -- Update original message
        local new_message = table.concat(message_parts, "\n")
        local new_ts = ctx:edit_message(ctx.reaction_target_timestamp, new_message)
        
        if new_ts and tostring(new_ts) ~= tostring(ctx.reaction_target_timestamp) then
            ctx:write_file("poll_" .. tostring(new_ts) .. ".txt", "ALIAS:" .. filename)
        end
    end
end
