# Coding Guidelines for Ceres Lua Packages

This is a coding guideline for Lua packages developed for usage in Ceres.

Since Lua itself doesn't have a standard best practices for the language itself, it is important to define a standard ourselves to that the code is consistent across the packages. This is especially important for the Standard Library.

## Rationale behind choices

The rationale behind a lot of these guidelines is to ensure consistency with Lua's built-in libraries and the Lua ecosystem in general. This pertains primarily to the naming of globals and libraries.

Other considerations come from my own opinions formed from extensive experience with Lua.

## Casing

Code should utilize `camelCase` for local variable names, method names, identifiers belonging to a library (functions, variables) and global functions.
`PascalCase` should be used for global variable names.
`SCREAMING_SNAKE_CASE` should be used for global variables that are intended to be used as constants. The full capitalization indicates that the variable should not be modified. It should also be used for custom macros.

## Libraries

A "library" is a table with functions (and sometimes also variables) with common functionality, exposed by a module. Some modules may also desire to expose the table as a global variable. Examples from Lua's standard library include libraries like `string` and `table`.

Ideally, libraries should have short, all-lowercase, concise names, to be consistent with Lua's standard library. If a library name is a noun, it should be singular. For example, a library that exposes functionality related to events should be called `event`, rather than `Event` or `events`. If it isn't possible to fit the library name into a single word, then `camelCase` should be used.

Other examples specific to Warcraft III:
* `unit`
* `region`
* `timer`
* `player`

Functions belonging to a library should use `camelCase`, e.g.:

* `region.create()`
* `unit.getAll()`
* `timer.simple()`
* `timer.repeating()`
* `player.getLocal()`

## Variable Names

Variable names should be descriptive and convey the true *intent* of the variable, rather than simply *what* it contains. Variable names **must not** be single-letter, except as loop counters. `for-each` loops should still prefer to use descriptive names, and discard unused variables with the `_` identifier.

Bad examples:
```lua
local u, unit

for k, v in pairs(t) do
    print(v.name)
end
```

Good examples:
```lua
local target, attacker

for _, victim in pairs(victims) do
    print(victim.name)
end
```

## Classes

If a module exposes a class, e.g. `Player`, then that class should **not** be exposed as a global `Player` variable. That class should only exist as a metatable assigned to objects returned from other, non-method functions.

I.e. prefer `unit.create()` over `Unit:create()`. This more clearly signals what is a free-standing function versus what is a class method. It does not make sense to use the `:method()` call syntax on non-method functions, and it can be potentially confusing to newcomers since it adds an implicit first `self` parameter, especially if it isn't used.

## Collections

Lua has a very powerful and performant built-in `table` type, which should be used for all your collection needs. Custom collection types rarely carry their weight in Lua, and should only be used when they provide a real benefit to usability. Avoid creating custom `Set`, `List`, `Array` or `Map` types. If you need to add operations that make tables behave like a queue, deque, stack or something else, add them to the `table` library, e.g.:

* `table.push()`
* `table.pop()`
* `table.pushBack()`
* `table.popBack()`

## Properties and Fields

Classes can expose fields or properties (getters/setters that look like fields, but are backed by an `__index` and a `__newindex` metamethod) for convenience where it makes sense to do so. A good candidate for this is a `Unit` class:

```lua
local myUnit = unit.create()
myUnit.x = 100
myUnit.y = 200
myUnit.maxHealth = 400
myUnit.health = 300
```

## Module Structure

Modules should follow the same naming rules as libraries - short, all-lowercase where possible, and `camelCase` if more verbosity is needed.

Avoid deeply nested modules. Sometimes, it may be useful to have a "blanket require" file which includes all the other members in the same module, or a certain subset of the functionality that may be useful. E.g. `require("std.basic")` and `require("std.all")`

If a library has a lot of different functionality, it should be possible to granurarily include only the parts of it that you need.

## Hooks and Initializers in Libraries

Libraries should avoid implicitly adding functionality to the map that may be considered "intrusive", especially if it has the potential to cause conflicts. In such cases, the user of the library should explicitly call the library's initializer in a hook of their own, possibly with some configuration. Excercise your best judgement when calling `ceres.addHook()` in library code!

## Error Handling

Lua has pretty decent error-handling capabilities and they should be used to their full extent in code. Public library functions should sanitize input where it makes sense to do so, and throw descriptive error messages if the contract is violated.

Functionality that takes user-provided callbacks should always use `pcall` or `xpcall` when calling them, and catch any errors that arise, re-throwing them further up or logging them.

Error logging should be configurable and be possible to turn off.

It may be possible to use macros in the near-future to entirely disable error-checking and reporting code from being compiled in Release builds.