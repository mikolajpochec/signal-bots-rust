description = "Search Wikipedia. Format: /wiki <query>"

local function urlencode(str)
    if str then
        str = string.gsub(str, "([^%w _%%%-%.~])",
            function(c) return string.format("%%%02X", string.byte(c)) end)
        str = string.gsub(str, " ", "_")
    end
    return str
end

local function get_json_string(json, key)
    local search = '"' .. key .. '":"'
    local start_idx = string.find(json, search, 1, true)
    if not start_idx then return nil end
    start_idx = start_idx + #search
    
    local res = {}
    local i = start_idx
    local len = #json
    while i <= len do
        local c = string.sub(json, i, i)
        if c == '\\' then
            i = i + 1
            local esc = string.sub(json, i, i)
            if esc == 'n' then table.insert(res, '\n')
            elseif esc == 't' then table.insert(res, '\t')
            elseif esc == '"' then table.insert(res, '"')
            elseif esc == '\\' then table.insert(res, '\\')
            elseif esc == 'u' then
                table.insert(res, '\\u')
            else
                table.insert(res, esc)
            end
        elseif c == '"' then
            break
        else
            table.insert(res, c)
        end
        i = i + 1
    end
    return table.concat(res)
end

function on_command(ctx)
    if not ctx.args or #ctx.args == 0 then
        ctx:reply("Please provide a search term.")
        return
    end

    local query = table.concat(ctx.args, " ")
    query = urlencode(query)

    local url = "https://en.wikipedia.org/api/rest_v1/page/summary/" .. query
    local response = ctx:http_get(url)

    if not response or response == "" then
        ctx:reply("No Wikipedia article found.")
        return
    end

    local extract = get_json_string(response, "extract")
    
    if extract and extract ~= "" then
        ctx:reply(extract)
    else
        ctx:reply("No Wikipedia article found.")
    end
end
