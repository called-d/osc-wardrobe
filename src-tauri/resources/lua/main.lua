local sys = require('init')

local function init()
    sys:init()
    setmetatable(wardrobe, {
        __newindex = function(t, k, v)
            rawset(t, k, v)
            if k == "definition" then
                sys:on_definition_changed()
            end
        end
    })
end

function main()
    init()
end

function receive(addr, args)
    sys:receive(addr, args)
    -- print(addr, table.unpack(args))
    if addr == "/avatar/change" then
        -- on avatar change
    else
        local alias = sys:find_avatar()
        if alias then
            local blueprint_id = wardrobe.definition.aliases[alias]
            if blueprint_id then
                print("alias found:", alias, "->", blueprint_id)
                local success, err = osc.send("/avatar/change", blueprint_id)
                if not success then
                    print("error on avatar change", err)
                end
            else
                print("alias found:", alias, "but blueprint_id not found")
            end
        end
    end
end
