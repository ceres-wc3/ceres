# Setup

Ceres doesn't offer a project initializer at the moment. Fortunately, the structure of a Ceres project is relatively simple.

First, create a directory for your project, with the following structure:

* `src/` - Script source files. Place your map scripts here.
* * `main.lua` - This is your map's entry point. All other scripts and resources should be required/included from this file.
* `lib/` - Script library files. Right now, this is equivalent to `src/`. In the future, this is where things like the standard library will go to, as well as any additional dependencies you have.
* `maps/` - Map sources. Each directory in this directory must be a valid WC3 map folder.

# Usage

Download the latest version for your platform from the (Releases)[https://github.com/ElusiveMori/ceres-wc3/releases] section. The easiest way is to place the executable in your project folder, however you can also place it in your `PATH` for convenience. 

To build a map, open a terminal in the project directory, and run:

`ceres build -- --map mymap.w3x`

This will look for the map in the `maps/` folder and inject it with the code processed from `src/` and `lib`/. 

To build **and run** the map:

`ceres run -- --map mymap.w3x`

# Module System

Ceres does some additional post-processing to add a Lua-like module system, which you are encouraged to take advantage of. To load a module, use:
```lua
require("mymodule")
```
This will look for the file `mymodule.lua`, first in the `lib/` directory, and then the `src/` directory, and include it into the final script if it's found.

If the module returns some value, e.g. a table with the module's functions, then that can be accessed using:
```lua
local mymodule = require("mymodule")
```

If you want to include a module which is inside a sub-directory, use the dot notation syntax for the module name:
```lua
local stuff = require("hello.world.stuff")
```

This will load the module in either `lib/hello/world/stuff.lua` or `src/hellow/world/stuff.lua`.

# Ceres built-ins

Ceres offers some small utilities for injecting code into the `main` and `config` functions of `war3map.lua`, or for overwriting them entirely.

```lua
-- add code to call before/after their respective entry points
ceres.addHook("main::before", function() ... end)
ceres.addHook("main::after", function() ... end)
ceres.addHook("config::before", function() ... end)
ceres.addHook("config::after", function() ... end)

-- override `main` and `config` entirely, without calling default code
-- Ceres hooks will still be called, however
ceres.setMain(function() ... end)
ceres.setConfig(function() ... end)
```
# Macros

At the moment, Ceres offers two additional macros, with more possibly coming in the future.

## `include()`

This macro will inject the contents of the specified file into the source where it is called. Paths are relative to the project's root, e.g. `include("src/resource/somestuff")`

## `compiletime()`

This macro will evaluate the given Lua expression at compiletime, and emit any resulting values into the source, if there were any.
If an argument is a function, then it is also executed.

```lua
print(compiletime(2 + 2)) -- compiles to print(4)
print(compiletime("my epic string" .. " really epic")) -- compiles to print("my epic string really epic")
print(compiletime(function()
    local a = 1
    local b = 2
    return a + b
end)) -- compiles to print(3)
```