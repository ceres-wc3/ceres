# Difference between Runtime and Compiletime

Ceres is built as a flexible Lua runtime with a default **build script** bundled in for the intended use-case - building Warcraft III maps. A **build script** is nothing more than a Lua script that Ceres runs to **build** the map. To **build** the map means to take in some inputs (Lua sources, an existing Warcraft III map), process them with the **build script** and produce an **artifact**. An **artifact** is either a single `war3map.lua` script, a "dirmap" or an MPQ archive.

Because Ceres itself runs using Lua, but also produces Lua scripts for usage in the map, it is important to distinguish between **runtime code** and **compiletime code**.

Runtime code is Lua code that gets included into the map itself. This is the code that Warcraft III will run when your map is being played, and Warcraft III APIs will be available as usual. However, Ceres' APIs are not available to runtime code, with the exception of a few small things that Ceres bundles into your map.

Compiletime code is Lua code that is run **by Ceres itself** to build the map and produce an artifact. Regular Warcraft III APIs are not available to compiletime code, and instead a set of compiletime APIs is available. Compiletime code can come from two sources:

* Built-in build scripts and custom build scripts, which Ceres runs when you invoke `ceres build` or `ceres run`
* `compiletime()` macros in Lua code that is meant for inclusion in the map, which get run while Ceres is processing the file

`compiletime()` macros and build scripts share the same environment, meaning that they have the same set of APIs available to them, and variables set in build scripts can be accessed (and set) in `compiletime()` macros, and vice-versa. Whether to use one or the other largely depends on where you think the code best belongs.

`compiletime()` macros also "inject" the result of their computation into the emitted Lua code.

# Build Process

Ceres has a relatively simple build process.

1. Arguments are parsed and the run mode is determined. A typical Ceres command will look like `ceres <command> [ceres args] -- <buildscript args>`. Notice the double dash `--`. Arguments before it are arguments to Ceres itself, while everything else will be passed on verbatim to the **build script**.

2. Ceres initializes a Lua runtime with WC3-specific extensions and APIs. It then loads a (Lua library)[../ceres-core/src/resources/buildscript_lib.lua] with some extra utilities and a default "build workflow". It then looks for a `build.lua` file in the working directory and runs that. After that, it runs `ceres.defaultHandler()`, which parses script arguments and kicks off the default build process, unless `ceres.suppressDefaultHandler()` has been called.

3. The default build workflow will try to determine the **input map** (`--map <mapname>`), and an **output type** (`--output <type>`). The input map is what Ceres will use as a "base" for producing a new map, and can be in either MPQ or directory format. The output type can be one of `mpq`, `dir`, or `script`.

4. Ceres compiles the map code, starting from `src/main.lua`, including any modules required by it into the final map script, running `compiletime()` expressions and so on. If the input map has a `war3map.lua` script, and the `--no-map-script` option has **not** been passed, then the map's script will also be included in the output.

5. Depending on the output type, it will either write the map as a directory or an MPQ archive, or simply output the compiled script into `target/war3map.lua`.