-- Build script utilities for Ceres

-- macro support
function define(id, value)
    if type(value) == "function" then
        ceres.registerMacro(id, value)
    else
        ceres.registerMacro(id, function()
            return value
        end)
    end
end

ceres.registerMacro("macro_define", define)