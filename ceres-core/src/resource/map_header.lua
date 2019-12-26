--[[ ceres map header start ]]
local ceres = {}
local __modules = {}


do
    local function print(...)
        local args = {...}
        local msgs = {}

        for k, v in pairs(args) do
            table.insert(msgs, tostring(v))
        end

        local msg = table.concat(msgs, " ")
        DisplayTimedTextToPlayer(GetLocalPlayer(), 0, 0, 60, msg)
    end

    local __ceres_hooks = {
        ["main::before"] = {},
        ["main::after"] = {},
        ["config::before"] = {},
        ["config::after"] = {}
    }

    local function __ceres_hookCall(hookName)
        for _, callback in ipairs(__ceres_hooks[hookName]) do
            callback()
        end
    end

    local __ceres_customMain
    local __ceres_customConfig

    local function __ceresMain()
        __ceres_hookCall("main::before")
        if __ceres_customMain ~= nil then
            __ceres_customMain()
        else
            ceres.__oldMain()
        end
        __ceres_hookCall("main::after")
    end

    local function __ceresConfig()
        __ceres_hookCall("config::before")
        if __ceres_customConfig ~= nil then
            __ceres_customConfig()
        else
            ceres.__oldConfig()
        end
        __ceres_hookCall("config::after")
    end

    function ceres.addHook(hookName, callback)
        table.insert(__ceres_hooks[hookName], ceres.wrapCatch(callback))
    end

    function ceres.setMain(callback)
        __ceres_customMain = callback
    end

    function ceres.setConfig(callback)
        __ceres_customConfig = callback
    end

    function ceres.catch(callback, ...)
        local success, err = pcall(callback, ...)

        if not success then
            print("ERROR: " .. err)
        end
    end

    function ceres.wrapCatch(callback)
        return function(...)
            ceres.catch(callback, ...)
        end
    end

    require = function(name)
        local module = __modules[name]

        if module ~= nil then
            if module.initialized then
                return module.cached
            else
                module.initialized = true
                module.cached = module.loader()
                return module.cached
            end
        else
            error("module not found")
        end
    end

    function ceres.init()
        ceres.__oldMain = main or function() end
        ceres.__oldConfig = config or function() end

        local success, error
        function main()
            if not success then
                print("|c00ff0000CRITICAL ERROR:|r Main map script failed to load:\n")
                print(tostring(error))
                print("Falling back to original map script.")
                ceres.__oldMain()
            else
                __ceresMain()
            end
        end
    
        function config()
            if error ~= nil then
                ceres.__oldConfig()
            else
                __ceresConfig()
            end
        end
    
        success, error = pcall(require, "main")
    end
end
--[[ ceres map header end ]]