local args = ceres.getScriptArgs()

local arg = {
    exists = function(arg_name)
        for _, v in pairs(args) do
            if v == arg_name then
                return true
            end
        end
        return false
    end,
    value = function(arg_name)
        local arg_pos
        for i, v in ipairs(args) do
            if v == arg_name then
                arg_pos = i
                break
            end
        end

        if arg_pos ~= nil and #args > arg_pos then
            return args[arg_pos + 1]
        end
    end
}

local manifest = {
    mapsDirectory = "maps/",
    srcDirectory = "src/",
    libDirectory = "lib/",
    targetDirectory = "target/"
}

if ceres.isManifestRequested() then
    ceres.sendManifest(manifest)
    return
end

local mapArg = arg.value("--map")
local mpqEnabled = arg.exists("--mpq")
local scriptOnlyEnabled = arg.exists("--script")

if scriptOnlyEnabled and mpqEnabled then
    error("--script and --mpq arguments are incompatible! Please choose one.")
end

if scriptOnlyEnabled and ceres.isRunmapRequested() then
    error("--script argument is incompatible with running a map, since there will be no map to run!")
end

if mapArg == nil and ceres.isRunmapRequested() then
    error("Running a map requires the --map argument to be present!")
end

local function compileScript(srcDirectory, libDirectory, map)
    local mapScript = nil

    if map ~= nil then
        mapScript = map:readFile("war3map.lua")

        if mapScript == nil then
            warn("The map does not contain a war3map.lua file.")
        else
            print("Loaded map script from " .. map:getBaseName())
        end
    end

    if mapScript == nil then
        mapScript = fs.readFile("src/war3map.lua")

        if mapScript == nil then
            warn("No map is set or the map does not contain a war3map.lua file, and there is no default (src/war3map.lua) file provided. No map script will be present in the output.")
        else
            print("Loaded map script from src/war3map.lua")
        end
    end

    print("Compiling script...")
    print("Src directory: " .. srcDirectory)
    print("Lib directory: " .. libDirectory)
    print("Map script: " .. (mapScript ~= nil and "present") or "not present")
    local script = ceres.compileScript {
        srcDirectory = srcDirectory,
        libDirectory = libDirectory,
        mapScript = mapScript
    }
    print("Compilation successful!")

    return script
end

local function writeBuildArtifact(script, map, writeAsMpq, targetDirectory)
    if map ~= nil then
        map:setScript(script)
    end

    local artifactPath

    if writeAsMpq then
        if map == nil then
            error("A map must be present to build an .mpq archive.")
        end

        if map:getType() == "folder" then
            warn("It is recommended to use a .w3x or .w3m file for the map when building an .mpq, otherwise some map information may not be present.")
        end

        artifactPath = targetDirectory .. map:getBaseName()
        print("Writing out map mpq archive to (" .. artifactPath .. ") ...")
        map:saveToMpq(artifactPath)
    else
        if map == nil then
            print("Writing out script file to (" .. (targetDirectory .. "war3map.lua") .. ")...")
            fs.writeFile(targetDirectory .. "war3map.lua", script)
        else
            artifactPath = targetDirectory .. "folder." .. map:getBaseName()
            print("Writing out map folder to (" .. artifactPath .. ") ...")
            map:saveToFolder(artifactPath)
        end
    end

    return artifactPath
end

local activeMap = nil

if mapArg ~= nil then
    print("Loading map (" .. mapArg .. ") ...")

    local mapPath = manifest.mapsDirectory .. mapArg
    local map, err = ceres.loadMap(mapPath)

    if map:isValid() then
        activeMap = map
    else
        error("A map build was requested, but the provided map (" .. mapPath .. ") was not valid. Reason: " .. err)
    end
end

local script = compileScript(manifest.srcDirectory, manifest.libDirectory, map)

if scriptOnlyEnabled then
    activeMap = nil
end

local artifactPath = writeBuildArtifact(script, activeMap, mpqEnabled, manifest.targetDirectory)

if ceres.isRunmapRequested() then
    ceres.runMap(artifactPath)
end