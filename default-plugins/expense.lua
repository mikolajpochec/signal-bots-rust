description = "Track group expenses. Format: /expense add <amount> <desc> | list | clear"
group = "Tools"

function on_command(ctx)
    -- Initialize table
    local init_sql = [[
        CREATE TABLE IF NOT EXISTS plugin_expenses (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            group_id TEXT,
            sender_name TEXT,
            amount REAL,
            description TEXT,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
        )
    ]]
    pcall(function() ctx:db_execute(init_sql, {}) end)

    if #ctx.args == 0 then
        ctx:reply("Usage: {::prefix}expense add <amount> <desc> | {::prefix}expense list | {::prefix}expense clear")
        return
    end
    
    local cmd = string.lower(ctx.args[1])
    local group_id = ctx.group_id or "DM_" .. ctx.sender_uuid
    local sender_name = ctx.sender_name or "Unknown"

    if cmd == "add" then
        if #ctx.args < 3 then
            ctx:reply("Usage: {::prefix}expense add <amount> <description>")
            return
        end
        local amount = tonumber(ctx.args[2])
        if not amount then
            ctx:reply("Amount must be a number.")
            return
        end
        local desc = table.concat(ctx.args, " ", 3)
        
        local status, err = pcall(function() 
            ctx:db_execute("INSERT INTO plugin_expenses (group_id, sender_name, amount, description) VALUES (?1, ?2, ?3, ?4)", {group_id, sender_name, tostring(amount), desc})
        end)
        
        if status then
            ctx:reply("✅ Added expense: " .. amount .. " for " .. desc .. " by " .. sender_name)
        else
            print("DB error: " .. tostring(err))
            ctx:reply("❌ Failed to add expense.")
        end
    elseif cmd == "list" then
        local status, results = pcall(function()
            return ctx:db_query("SELECT sender_name, SUM(amount) FROM plugin_expenses WHERE group_id = ?1 GROUP BY sender_name ORDER BY SUM(amount) DESC", {group_id})
        end)
        
        if not status or not results then
            ctx:reply("No expenses recorded yet, or an error occurred.")
            return
        end
        
        if #results == 0 then
            ctx:reply("No expenses recorded yet.")
            return
        end
        
        local msg = "💸 **Group Expenses** 💸\n"
        for _, row in ipairs(results) do
            msg = msg .. row[1] .. ": " .. row[2] .. "\n"
        end
        ctx:reply(msg)
    elseif cmd == "clear" then
        local status, err = pcall(function()
            ctx:db_execute("DELETE FROM plugin_expenses WHERE group_id = ?1", {group_id})
        end)
        if status then
            ctx:reply("✅ Cleared all expenses for this group.")
        else
            ctx:reply("❌ Failed to clear expenses.")
        end
    else
        ctx:reply("Unknown subcommand. Use add, list, or clear.")
    end
end
