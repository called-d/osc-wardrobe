
function main()
    local sec = 5.5
    sleep(sec)
    return sec
end

function receive(addr, args)
    print(addr, table.unpack(args))
end