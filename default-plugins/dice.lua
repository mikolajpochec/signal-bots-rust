description = "Roll a dice (e.g. !dice 6)"

function on_command(ctx)
    local sides = tonumber(ctx.args[1]) or 6
    if sides < 1 then sides = 6 end
    local result = math.random(1, sides)
    ctx:reply("🎲 You rolled a " .. tostring(result) .. " (d" .. tostring(sides) .. ")")
end
