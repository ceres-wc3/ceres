# 0.3.4

* Attempted to fix an issue where sometimes field types in the OE API would not resolve correctly, causing some fields (such as `weapsOn`) to not work correctly.

# 0.3.3

* `fs.watchFile` no longer blocks. instead, Ceres spins up an event loop after the build script finishes where it processes file watchers started by `fs.watchFile`. The event loop will terminate if a WC3 instance launched by Ceres has exited. If WC3 wasn't started, then the event loop will continue running indefenitely until manually terminated.
* The object API now has `getObject`, `setObject`, `getField` and `setField` methods for object storages and objects. They function identically to their indexing counterparts, and are meant for usage in TypeScript where the type system cannot correctly express the types returned from indexing operations in all cases.
* `-launch` is now automatically appended to WC3 arguments when running via Ceres.
* Files added via `map:addFileString`, `map:addFileDisk`, and `map:addDir` can now be read back via `map:readFile`. Previously, `map:readFile` would only return files that already existed in the map mpq/dir.

# 0.3.2

* Fixed a small bug where Ceres would not quit after building, even when no Live Reload was enabled.
* Removed all unstable feature gates and dependencies. Ceres now compiles on stable Rust.

# 0.3.1

## Breaking
* `ceres.catch` and `ceres.wrapCatch` were renamed to `ceres.safeCall` and `ceres.wrapSafeCall` to avoid clashing with the `catch` operator in TS
* Ceres no longer suppresses default `main` and `config` functions if their respective modules returned a non-false result. Instead, if you want to suppress default `main` and `config` behaviour, you can call `ceres.suppressDefaultMain()`, and `ceres.suppressDefaultConfig()`. This was a particular pain point for TS users with a `main.ts` module.
* `mpq.new` was renamed to `mpq.create` to avoid clashing with the `new` operator in TS
* Replaced `ceres.layout.srcDirectory` and `ceres.layout.libDireclctory` with one array - `ceres.layout.srcDirectories`, allowing you to specify any number of source directories to instruct Ceres to look in. This is useful for TypeScript projects that can have a pure-Lua directory, a compiled TS directory, and external dependencies in `node_modules`.

## Non-Breaking
* When invoking `ceres run`, Ceres will now wait for WC3 to exit before shutting down. This is useful in VS Code on certain platforms, where previously a finished VS Code task running `ceres run` would make WC3 exit immediately.
* Added a `fs.copyFile` function.
* Added a `map:addDir` method to `map` objects, allowing you quickly import an entire directory into the map.
* Pulled in upstream bugfixes from `ceres-mpq` related to path separator issues
* Added a Ceres-specific unit field called `siid` to the Object API, which returns the unit's editor name, which is used by natives like `CreateUnitByName`

## Documentation

The documentation for Ceres has been updated. There are now template repositories for Lua and TypeScript, as well as a [TypeScript library](https://github.com/ceres-wc3/cerrie) which provides idiomatic wrappers over JASS natives and useful utilities such as File I/O and Live Reload.

Check the readme for more information. 

# 0.3.0

__Significant breaking changes in this release!__

## Breaking changes
* Changed the way the generated map script works. Now all code `require`d through `main.lua` will run through the `main` function, permitting full safe access to all natives. Previously, you'd have to tip-toe around which WC3 systems were initialized at script-load time and which weren't. Now you can simply call those functions in the script body, without the need for hooks or any other tricks.
* As a consequence, it is no longer possible to add/replace code in the `config` function through `main.lua`. If you still want to run code in the `config` section of the map script, create a `config.lua` file. The contents of this file will be executed in WC3's `config` section.
* The same applies to init-time loading. If you want to run some code before either `config` or `main` execute, create a file called `init.lua`.
* If you want to suppress the default behaviour of `main` or `config` (e.g. if you are doing all map initialization and player setup yourself), then `return true` in the respective files. Otherwise, your code will run before the default actions.
* As a consequence, Ceres hooks `main::after`, `main::before`, `config::after`, and `config::before` no longer exist. Ceres will throw an error if you try to add a hook with either of those names.
## New features
* The default map script template now has preliminary support for live-reloading. Please note that this __is an unfinished feature__. However, the script will automatically detect if it has been run a second time, and fire the `reload::before` and `reload::after` hooks when it does so, as well as reloading all the modules within itself.
* Ceres now loads individual modules as a string rather than as a function, which allows WC3 to correctly report errors as located inside the offending module, rather than simply reporting them as being inside `war3map.lua`.
* Added a new function `fs.watchFile(path, callback)`, which will execute `callback` with a file's new contents when the said file changes. This can be used to communicate with WC3 by creating Preloader files from inside WC3 and loading a response file inside WC3. This will be used by live reload.
* Modules can now be specified as optional, i.e. `require('mymodule', true)`. Ceres will simply ignore it if the module does not exist.

## Bug fixes
* Fixed a bug where certain unit fields were incorrectly parsed in the metadata, causing them to throw errors upon access attempts.
* Fixed a bug in the Lua parser failing to parse `elseif` clauses sometimes.

# 0.2.5

* Fixed a bug with script-only compilation
* Made some adjustments to the default build script and header file
* Bumped the version of `ceres-mpq` to 0.1.6, fixing some bugs in the MPQ implementation

# 0.2.4

### Object data manipulation
* Build scripts (as well as compiletime macros) can now manipulate object data of the map
* Default English WC3 data is provided for introspection purposes

The default build script sets a global variable `currentMap` which exposes the `map` object of the currently processed map.

Inside `compiletime()` expressions you can now access this variable, and edit objects via an intuitive table-like syntax. For example:
```lua
compiletime(function()
    local customFootman = currentMap.objects.unit['hfoo']:clone()
    customFootman.Name = "A Custom Footman"
    currentMap.objects.unit['x000'] = customFootman
end)
```

### Other

* Various bugs have been fixed with the build pipeline

# 0.2.2

* Fixed various issues which broke Ceres

# 0.2.1

* Fixed a bug where the Lua compiler would ignore `libDirectory`

# 0.2.0

Initial release of the Lua-centric rewrite.