# 0.2.3

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