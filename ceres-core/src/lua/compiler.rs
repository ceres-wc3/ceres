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

    let mut module_provider = compiler::ProjectModuleProvider::new(
        src_directory.to_str().unwrap().into(),
        lib_directory.to_str().unwrap().into(),
    );

    module_provider.scan();

    let macro_provider = macros::get_threadlocal_macro_provider();

    let mut compiler = compiler::ScriptCompiler::new(ctx, module_provider, macro_provider);

    compiler.set_map_script(map_script.to_str()?.into());
    compiler.add_module("main").map_err(LuaError::external)?;

    Ok(compiler.emit_script())
}
