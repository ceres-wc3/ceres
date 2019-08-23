use rlua::prelude::*;

use crate::compiler;
use crate::lua::macros;

pub fn get_compile_script_luafn(ctx: LuaContext) -> LuaFunction {
    ctx
        .create_function::<LuaTable, String, _>(|ctx, args: LuaTable| {
            let src_directory: LuaString = args.get("srcDirectory").unwrap();
            let lib_directory: LuaString = args.get("libDirectory").unwrap();

            let map_script: LuaString = args.get("mapScript").unwrap();

            let mut module_provider = compiler::ProjectModuleProvider::new(
                src_directory.to_str().unwrap().into(),
                lib_directory.to_str().unwrap().into(),
            );

            module_provider.scan();

            let macro_provider = macros::get_threadlocal_macro_provider();

            let mut compiler = compiler::ScriptCompiler::new(ctx, module_provider, macro_provider);

            compiler.set_map_script(map_script.to_str().unwrap().into());
            compiler.add_module("main").map_err(LuaError::external)?;

            Ok(compiler.emit_script())
        })
        .unwrap()
}
