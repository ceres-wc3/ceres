**WARNING: The master branch currently hosts the 0.2.x version of Ceres which is largely untested and potentially unstable. Documentation is WIP and is incomplete. If you're looking for the old 0.1.x version, click [here](https://github.com/ElusiveMori/ceres-wc3/tree/v0.1.5)**

# About

Ceres is a build toolchain for **Warcraft III Lua Maps**. Its primary goal is to allow **editing maps in the comfort of your code editor of choice**, while also using the opportunity to provide additional utilities to mappers that are currently not present in World Editor, or unlikely to ever be introduced into World Editor.

If you just want to get started with making a Warcraft III map using Lua, skip to [Setup](docs/setup.md).

To be more precise, Ceres in a **bundled Lua runtime** with facilities and libraries specific to Warcraft III map development. Most operations in Ceres actually run various Lua scripts, with extensions provided by Ceres itself, such as reading and writing of MPQ archives, filesystem access, a script compiler for WC3 maps, and so on.

Ceres comes bundled with a default **build script** written in Lua which aims to provide a standard, configurable workflow for building a Warcraft III map. The build script is configurable and extensible, and if you wish you can just run an entirely unrelated Lua script using Ceres' built-in libraries. It's built to be useful as more than just a compiler.

Because everything is bundled together into one executable, you don't need to fuss around with external requirements, dependency managers, and anything else. Ceres is meant to be portable and easy to get running.

# Documentation

Documentation is a bit sparse at the moment, however, I'm trying to fill the gaps as I go.

[Manual](docs/manual.md)  
[Compiletime APIs](docs/compiletime.md)  
[Setup Guide](docs/setup.md)  

# Current Status

At the moment, Ceres is able to compile multiple Lua files into a single Lua script for distribution in Warcraft III maps, and has a framework around the compiler to facilitate the map build process. It can read and write MPQ archives for final distribution of a map.

The compiler is currently able to:
* Resolve module dependencies and compile it all down into a single Lua file
* Process custom macros and evaluate Lua code during the build of a map

The second point allows you to run arbitrary Lua code while the map is compiling. Right now this feature isn't terribly useful, however in the future it will allow to edit the map being compiled to add custom units, spells, set map description, etc. from within Lua scripts.

# Roadmap

- [x] Script compiler
- [x] Macro system
- [x] Lua-based build system
- [x] MPQ read/write API
- [x] APIs to edit map data (objects, description, title, imports, etc.)
- [ ] Standard WC3 Lua library 
- [ ] Dependency manager (likely via git)
- [ ] Beginner tutorial
- [ ] Documentation of APIs provided by Ceres and advanced usage
- [ ] VSCode integration via an extension
- [ ] New project template
- [ ] Auto-updates within Ceres (or via the VSCode extension)
