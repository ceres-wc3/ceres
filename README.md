# **!!!Notice!!!**

Ceres is discontinued indefinitely. This is mainly due to my unavailability, and the complete dumpster fire that Reforged turned out to be.

If you wish to pick up the project, the license permits you to do so.

# About

Ceres is a stand-alone scriptable build toolchain for Warcraft III maps. It provides a way to quickly and easily build and package Warcraft III Lua maps, as well as various other utilities and tools.

It is scriptable and customizable using Lua, and can be used to orchestrate whatever you want, including:
- Building a single Lua map
- Building multiple Lua maps
- Editing your maps' Object Data using Lua
- Managing imports
- And so on

## Quick Start

Ceres can be used with pure Lua, or with TypeScript using a [TypeScript to Lua transpiler](https://github.com/TypeScriptToLua/TypeScriptToLua). There are template repositories for both.

### Pure Lua

If you just want to use Lua and nothing else, setup is very minimal. Clone [this repository](https://github.com/ceres-wc3/ceres-lua-template), follow the instructions in the readme, and customize it to your heart's content.

### TypeScript

If you want to use TypeScript in your maps, Ceres can be used with it too. Setup is a bit more complex than with pure Lua, but you can use [this repository](https://github.com/ceres-wc3/ceres-ts-template) as a template. Make sure to read the readme.

There is also an example of using NPM to download external dependencies in your project, namely [Cerrie](https://github.com/ceres-wc3/cerrie), which is a library providing an idiomatic set of APIs for TypeScript projects, as well as some utilities such as File I/O and Live Reload. If you want to get started with Cerrie, take a look at [this branch of `ceres-ts-template`](https://github.com/ceres-wc3/ceres-ts-template/tree/cerrie). 

## API & Docs

Ceres provides various APIs to enable it to do what it does, to both maps and build scripts. Namely, it has APIs for object editing, MPQ reading/writing, file I/O, Lua script compilation, file preprocessing and so on. The entire API surface has been documented in the form of a [TypeScript declaration file](https://github.com/ceres-wc3/ceres-decl), which you can use as a reference even when not using TypeScript - all APIs are themselves pure Lua and do not require TypeScript.

Parts of Ceres are also documented more in-depth in the Wiki, which you can check out for extra information.

## Build Process

Ceres works by running a Lua script to build your map, called a *build script*. There is a default configuration that takes a map from a configurable folder and augments it with. If there is a `build.lua` file in the root of your project, it will also run that before building the map, allowing you to configure the build process, or override it entirely.

Lua code is analyzed by looking at `require` calls to bundle all required modules into one Lua file - since Warcraft III doesn't support multiple Lua files yet.

Code can also be executed during the build process by means of the `compiletime` macro. 

For example:

```lua
--[[ inside build.lua ]]

-- let's setup some data
function getSomeData()
    return {1,2,3,4,5}
end

--[[ inside main.lua ]]
local a = compiletime(function()
    print("This will be printed during the build process!")
    
    -- all compiletime macros execute in the same Lua context as other macros
    -- and `build.lua`, meaning you can share data and functions between them
   
    local data = getSomeData()
    return data
end)

-- compiletime will embed its return value into the compiled code,
-- allowing you to retain information from the build stage
-- will print '5'
print(a[5])
```

## Macros

Ceres preprocesses your Lua files before including them in the final script, allowing you to execute built-in and custom macros. The `compiletime()` macro has been mentioned before - it simply executes some code (or calculates a provided expression) and embeds the result as a Lua value in the resulting script. There is also `include`, which simply embeds a file into the source code with no preprocessing.

You can also register custom macros using `macro_define`. As a rather complex example:
```lua
compiletime(function()
    -- this strips out all newlines and extra whitespace from the macro
    function prepare_macro_template(s)
        return s:gsub("\n", ""):gsub("[ ]+", " "):gsub("^[ ]+", ""):gsub("[ ]+$", "")
    end

    -- create a template for our macro
    -- this code will be injected into the final script
    ASSERT_ARG_TEMPLATE = prepare_macro_template([[
        do
        local t = typeId(%s)
        if t ~= "%s" then
            print(t)
            error("argument #%s ('%s') must be a '%s', but got a " .. t .. " instead")
        end
        end
    ]])
   
    -- define a global function at compiletime which takes in the macro arguments and returns a string which will be embeded in the code
    function MAKE_ASSERT_ARG_TYPE(num, arg, requiredType)
        -- asserts can be disabled by setting the ASSERTS_DISABLED variable during the build process
        -- if they are disabled, nothing will be embedded in the code
        if not ASSERTS_DISABLED then
            -- if the asserts aren't disabled, return the formatted macro template according to our args
            return string.format(ASSERT_ARG_TEMPLATE, arg, requiredType, num, arg, requiredType)
        else
            return ""
        end
    end
end)

-- macro_define is itself a macro - it registers a custom macro handler
-- which will be invoked when the macro is encountered by the preprocessor
-- the first argument is the macro name
-- the second argument is the macro handler. you can provide a closure here,
-- but in our case we provide MAKE_ASSERT_ARG_TYPE, which we have defined previously
macro_define("ASSERT_ARG_TYPE", MAKE_ASSERT_ARG_TYPE)

-- example usage
-- asserts that a and b are numbers, otherwise throws an error
-- allows disabling asserts by enabling ASSERTS_DISABLED
function add(a, b)
    ASSERT_ARG_TYPE(1, "a", "number")
    ASSERT_ARG_TYPE(2, "b", "number")
    
    return a + b
end
```
