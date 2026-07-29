description = "System lifecycle plugin (not invoked by users)"
aliases = { verbose = "Toggle lifecycle notifications." }

local function get_chat_id(ctx)
    if ctx.is_group then
        return "GROUP|" .. ctx.group_id
    else
        return "USER|" .. (ctx.sender_uuid or ctx.sender_number)
    end
end

local function get_subscribed_chats(ctx)
    local content = ctx:read_file("verbose_chats.txt")
    if not content or content == "" then return {} end
    local chats = {}
    for line in content:gmatch("[^\r\n]+") do
        chats[line] = true
    end
    return chats
end

local function save_subscribed_chats(ctx, chats)
    local content = ""
    for chat, _ in pairs(chats) do
        content = content .. chat .. "\n"
    end
    ctx:write_file("verbose_chats.txt", content)
end

function on_command(ctx)
    if ctx.trigger == "verbose" then
        local action = ctx.args[1]
        if action == "on" then
            local chats = get_subscribed_chats(ctx)
            local chat_id = get_chat_id(ctx)
            chats[chat_id] = true
            save_subscribed_chats(ctx, chats)
            ctx:reply("Lifecycle notifications (spawn/death) enabled for this chat.")
        elseif action == "off" then
            local chats = get_subscribed_chats(ctx)
            local chat_id = get_chat_id(ctx)
            chats[chat_id] = nil
            save_subscribed_chats(ctx, chats)
            ctx:reply("Lifecycle notifications disabled for this chat.")
        else
            ctx:reply("Usage: {::prefix}verbose on|off")
        end
    end
end

local function broadcast_to_subscribers(ctx, msg)
    local chats = get_subscribed_chats(ctx)
    for chat, _ in pairs(chats) do
        local is_group = chat:sub(1, 6) == "GROUP|"
        local id = chat:sub(7)
        if is_group then
            ctx:send_group_message(id, msg)
        else
            ctx:send_message(id, msg)
        end
    end
end

function on_spawn(ctx)
    local date_str = os.date("%Y-%m-%d %H:%M:%S")
    local msg = "🟢 **Bot Started**\nSystem online at: " .. date_str
    broadcast_to_subscribers(ctx, msg)
end

function on_death(ctx)
    local date_str = os.date("%Y-%m-%d %H:%M:%S")
    local uptime = ctx.bot_uptime
    local hours = math.floor(uptime / 3600)
    local mins = math.floor((uptime % 3600) / 60)
    local secs = uptime % 60
    
    local uptime_str = ""
    if hours > 0 then
        uptime_str = uptime_str .. hours .. "h "
    end
    if mins > 0 or hours > 0 then
        uptime_str = uptime_str .. mins .. "m "
    end
    uptime_str = uptime_str .. secs .. "s"

    local msg = "🔴 **Bot Shutting Down**\nTime: " .. date_str .. "\nUptime: " .. uptime_str
    broadcast_to_subscribers(ctx, msg)
end
