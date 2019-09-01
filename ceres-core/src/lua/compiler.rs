use rlua::prelude::*;

use crate::compiler;
use crate::lua::macros;
use crate::lua::util::lua_wrap_result;
use crate::error::AnyError;

pub fn get_compile_script_luafn(ctx: LuaContext) -> LuaFunction {
    ctx.create_function(|ctx, args: LuaTable| {
        let result = compile_script(ctx, args);

        Ok(lua_wrap_result(ctx, result))
    })
    .unwrap()
}

fn compile_script(ctx: LuaContext, args: LuaTable) -> Result<String, AnyError> {
    let src_directory: LuaString = args.get("srcDirectory")?;
    let lib_directory: LuaString = args.get("libDirectory")?;

    let map_script: LuaString = args.get("mapScript")?;

    unimplemented!()
}
