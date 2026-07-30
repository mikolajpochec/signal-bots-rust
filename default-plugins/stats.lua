description = "Show the top 5 most active chatters in this group. Format: {::prefix}stats"
group = "Tools"

function on_command(ctx)
    if not ctx.is_group then
        ctx:reply("This command only works in groups!")
        return
    end

    local status, results = pcall(function()
        return ctx:db_query("SELECT sender_name, COUNT(*) as count FROM messages WHERE group_id = ?1 GROUP BY sender_uuid ORDER BY count DESC LIMIT 5", {ctx.group_id})
    end)
    
    if not status or not results or #results == 0 then
        ctx:reply("No stats available yet, or an error occurred.")
        return
    end
    
    local msg = "📊 **Top Chatters** 📊\n"
    for i, row in ipairs(results) do
        local name = row[1]
        if name == "NULL" or name == "" then
            name = "Unknown User"
        end
        local count = row[2]
        msg = msg .. i .. ". " .. name .. ": " .. count .. " messages\n"
    end
    
    ctx:reply(msg)
end
