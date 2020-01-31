--[[ ceres map header start ]]
ceres = ceres or {}
ceres.modules = {}

ceres.initialized = ceres.initialized or false

do
    function _G.print(...)
        local args = {...}
        local msgs = {}

        for k, v in pairs(args) do
            table.insert(msgs, tostring(v))
        end

        local msg = table.concat(msgs, " ")
        DisplayTimedTextToPlayer(GetLocalPlayer(), 0, 0, 60, msg)
    end

    ceres.hooks = ceres.hooks or {
        ["reload::before"] = {},
        ["reload::after"] = {},
    }

    function ceres.hookCall(hookName)
        for _, callback in ipairs(ceres.hooks[hookName]) do
            callback()
        end
    end

    function ceres.addHook(hookName, callback)
        if not ceres.hooks[hookName] then
            error(("can't register non-existent Ceres hook '%s'"):format(hookName))
        end

        table.insert(ceres.hooks[hookName], ceres.wrapSafeCall(callback))
    end

    function ceres.safeCall(callback, ...)
        local success, err = pcall(callback, ...)

        if not success then
            print("ERROR: " .. err)
        else
            return err
        end
    end

    function ceres.wrapSafeCall(callback)
        return function(...)
            ceres.safeCall(callback, ...)
        end
    end

    _G.require = function(name, optional)
        local module = ceres.modules[name]

        if module ~= nil then
            if module.initialized then
                return module.cached
            else
                module.initialized = true
                local compiled, err = load(module.source, "module " .. name)
                if not compiled then
                    error("failed to compile module " .. name .. ": " .. err)
                end

                module.cached = compiled()
                return module.cached
            end
        elseif not optional then
            error("module not found")
        end
    end

    function ceres.init()
        if not ceres.initialized then
            ceres.oldMain = main or function() end
            ceres.oldConfig = config or function() end

            local success, err
            function _G.main()
                if ceres.modules["init"] and not success then
                    print("|c00ff0000CRITICAL ERROR:|r Init script failed to load:\n")
                    print(err)
                end

                if ceres.modules["main"] then
                    local result = ceres.safeCall(require, "main")
                    if not result then
                        ceres.safeCall(ceres.oldMain)
                    end
                else
                    ceres.safeCall(ceres.oldMain)
                end

                ceres.initialized = true
            end

            function _G.config()
                if ceres.modules["config"] then
                    local result = ceres.safeCall(require, "config")
                    if not result then
                        ceres.safeCall(ceres.oldConfig)
                    end
                else
                    ceres.safeCall(ceres.oldConfig)
                end
            end

            if ceres.modules["init"] then
                success, err = pcall(require, "init")
            end
        else
            ceres.hookCall("reload::before")
            ceres.hooks["reload::before"] = {}
            ceres.hooks["reload::after"] = {}
            local success, error = pcall(require, "main")
            if not success then
                print("|c00ff0000CRITICAL ERROR:|r Main map script failed to REload:\n")
                print(tostring(error))
                return
            end
            ceres.hookCall("reload::after")
        end
    end
end
--[[ ceres map header end ]]