local function init()
    setmetatable(wardrobe, {
        __newindex = function(t, k, v)
            rawset(t, k, v)
            if k == "definition" then print("[wardrobe bridge] definition changed") end
        end
    })
end

function main()
    init()
    local sec = 5.5
    sleep(sec)
    local success, err = osc.send("/avatar/change", "avtr_00000000-0000-4000-0000-000000000000")
    if not success then
        print("error on avatar change", err)
    end
    return sec
end

function receive(addr, args)
    print(addr, table.unpack(args))
end