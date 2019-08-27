Ceres provides various APIs useful for Warcraft III development for usage in build scripts and compiletime expressions.  
Keep in mind that these are **only available in Ceres itself, they will not be available in map code**.

# Error handling
If a function or a method can fail in a non-critical way, then its first return value will be either its intended return value (or `true`, in case it doesn't make sense to return anything else), or `false`, if the operation failed, followed by an error message as the second return value.

For example:
```
local archive, errorMsg = mpq.open("myarchive.mpq")
if not archive then
    print("An error occured: " .. errorMsg)
end
```

# MPQ Library
Ceres provides a simple MPQ library that supports reading and writing MPQ archives.

## Reading an archive
### `mpq.open(path: string) -> archive, errorMsg`
Opens an MPQ archive for reading. Returns an `archive` object if successful.

### `archive:readFile(path: string) -> string, errorMsg`
Reads a single file from the archive. Returns the contents as a `string`.

### `archive:files() -> table`
Reads the archive's listfile, if there is one. Returns `nil` if no listfile is present or it couldn't be read.

Note: This is different than simply doing `archive:readFile("listfile")`: 
1. It will parse the listfile and return it as a table of file names.
2. It will convert backward slashes (`\\`) to forward slashes (`/`) in file names.
### `archive:header() -> string, errorMsg`
This will read an archive's **file header**, which is a portion of the file directly preceding the archive's MPQ header. This is useful for e.g. Warcraft III maps (`.w3m`, `.w3x`) because they contain one such header which is not part of the MPQ spec, but since Warcraft III requires the header to be present in order to read the map, this utility function is included here.

## Writing an archive
### `mpq.new() -> builder`
Creates a new "MPQ Builder" object which can be used to create new archives.
### `builder:add(path: string, contents: string, options: table) -> boolean, errorMsg`

Adds a single file to the builder, where the `contents` of the file are provided as a Lua string. The `options` table is optional, and can be used to specify whether to encrypt and compress the file:
```
{
    encrypt = false,
    compress = true
}
```
The default is to compress, but not encrypt.
### `builder:add(path: string, filePath: string, options: table) -> boolean, errorMsg`
Same as `archive:add()`, but instead of passing in the file contents via a lua string, will instead try to add the file directly from the disk.

### `builder:setHeader(header: string)`
Sets a file header to be prepended before the archive. For an explanation, see `archive:header()`