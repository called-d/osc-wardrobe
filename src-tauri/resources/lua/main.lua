
function main()
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