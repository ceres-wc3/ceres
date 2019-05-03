# About

Ceres is a Warcraft III map builder and code preprocessor for Lua-based maps.
Runs on Windows and Linux.

# Current Features:

* Convenient project folder structure 
* Building and running the map in WC3 from the command line
* Combines code from `.lua` files into a single `war3map.lua`
* Lua-like `require`-based module system built-in
* Basic macro support - `include` and `compiletime`

# Setup

At the moment, you will have to set everything up manually.

First, create a Ceres project folder, with the following structure:

* `src/` - Script source files. Place your map scripts here.
* * `main.lua` - This is your map's entry point. All other scripts and resources should be required/included from this file.
* `lib/` - Script library files. Right now, this is equivalent to `src/`. In the future, this is where things like the standard library will go to, as well as any additional dependencies you have.
* `maps/` - Map sources. Each directory in this directory must be a valid WC3 map folder.
* `ceres.toml` - Ceres configuration file. For an example of a default config file, look below. Currently, this file is mandatory, and Ceres won't work if it's not found.

# Configuration

In order to specify various configuration values, Ceres looks for a `ceres.toml` file in the project directory or for a `$HOME/.ceres/config.toml` file.

## Windows Configuration
For usage on Windows, you only need the following:
```toml
[run]
wc3_start_command = "<PATH TO WC3 EXECUTABLE>"
```

You can also specify which window mode to use:
```toml
[run]
wc3_start_command = "<PATH TO WC3 EXECUTABLE>"
window_mode = "windowedfullscreen" # other possible values: fullscreen, windowed
```

## Linux Configuration
For usage on Linux, you will also need to specify that you're running WC3 under wine and what path prefix to use to locate the map.
```toml
[run]
wc3_start_command = "start-wc3"
is_wine = true
wine_disk_prefix = "Z:"
window_mode = "windowedfullscreen"
```

In this example, `start-wc3` is a custom script that launches Warcraft III in it's own `WINEPREFIX`. 

# Usage

Currently, Ceres only supports command-line usage. Make sure that the `ceres` executable is either in your `PATH`, or otherwise easily accessible.

To build a map, `cd` into the project directory, and run:

`ceres build mymap.w3x`

This will look for the map in the `maps/` folder and inject it with the code processed from `src/` and `lib`/. Currently, Ceres only supports folder-based maps (WC3 1.31 PTR feature).

To build **and run** the map:

`ceres run mymap.w3x`

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