--[[
local function table_keys(t)
    local keys = {}
    for k, _ in pairs(t) do
        table.insert(keys, k)
    end
    return keys
end

local function print_conditions(conditions, depth)
    depth = depth or 0
    if type(conditions) ~= "table" then
        print(string.rep("  ", depth) .. conditions)
        return
    end
    for _, t in ipairs(conditions) do
        local v = type(t.v) == "table" and "" or t.v
        print(string.rep("  ", depth) .. "cond: " .. table.concat(table_keys(t.cond), " and "), "->", v)
        if type(t.v) == "table" then
            print_conditions(t.v, depth + 1)
        end
    end
end
--]]

local function is_superset_keys(a, b)
    for k, _ in pairs(b) do
        if not a[k] then
            return false
        end
    end
    return true
end

local function split(str, sep)
    local result = {}
    for match in (str .. sep):gmatch("(.-)" .. sep) do
        table.insert(result, match)
    end
    return result
end

local function parse_condition(cond)
    local result = {}
    if cond == "*" then return result end
    for _, v in ipairs(split(cond, " and ")) do
        v = v:gsub(" == ", "="):gsub(" = ", "=")
        result[v] = true
    end
    return result
end

local function build_conditions_list(conditions)
    local result = {}
    for cond, v in pairs(conditions) do
        if type(v) == "table" then
            v = build_conditions_list(v)
        end
        table.insert(result, { cond = parse_condition(cond), v = v })
    end
    table.sort(result, function (a, b)
        if is_superset_keys(a.cond, b.cond) then
            return true
        else
            return #a.cond > #b.cond
        end
    end)
    return result
end

local function find(conditions, parameters)
    for _, t in ipairs(conditions) do
        if is_superset_keys(parameters, t.cond) then
            local v = t.v
            if type(v) == "table" then
                local r = find(v, parameters)
                if r then return r end
            else
                return v
            end
        end
    end
    return nil
end

local processor = {
    avatar_context = {
        parameters = {},
        conditions = {},
    }
}

local function onavatarchange(id, keep_parameters)
    local avatar = wardrobe.definition.avatars[id]
    processor.avatar_context = {
        id = id,
        parameters = keep_parameters and processor.avatar_context.parameters or {},
        conditions = build_conditions_list(avatar or {})
    }
    if not id then return end

    print("[wardrobe bridge] onavatarchange", id)
    if not avatar then
        print("[wardrobe bridge] no avatar found for id", id)
    end
end

function processor:init()
    --
end
function processor:on_definition_changed()
    print("[wardrobe bridge] definition changed")
    onavatarchange(self.avatar_context.id, true)
end
function processor:receive(addr, args)
    if addr == "/avatar/change" then
        onavatarchange(args[1])
    else
        self.avatar_context.parameters[addr] = args[1]
    end
end
function processor:find_avatar()
    local parameters = {}
    for k, v in pairs(self.avatar_context.parameters) do
        parameters[k.."="..(v or "")] = true
    end
    return find(self.avatar_context.conditions, parameters)
end


return processor
