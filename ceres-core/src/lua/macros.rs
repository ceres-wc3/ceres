use rlua::prelude::*;

use std::collections::HashMap;
use std::thread_local;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;

use crate::compiler;

use crate::lua::util::evaluate_macro_args;
use crate::lua::util::value_to_string;

use compiler::MacroProvider;

pub struct LuaMacroProvider {
    registered_macros: HashMap<String, LuaRegistryKey>,
}

impl LuaMacroProvider {
    fn register_macro<'lua>(&mut self, ctx: LuaContext<'lua>, id: &str, func: LuaFunction<'lua>) {
        let registry_key = ctx.create_registry_value(func).unwrap();

        self.registered_macros.insert(id.into(), registry_key);
    }
}

impl MacroProvider for LuaMacroProvider {
    fn is_macro_id(&self, id: &str) -> bool {
        self.registered_macros.contains_key(id)
    }

    fn handle_macro(
        &self,
        ctx: LuaContext,
        id: &str,
        compilation_data: &mut compiler::CompilationData,
        macro_invocation: compiler::MacroInvocation,
    ) {
        let args = evaluate_macro_args(ctx, macro_invocation.args).unwrap();

        let registry_key = self.registered_macros.get(id).unwrap();
        let callback: LuaFunction = ctx.registry_value(registry_key).unwrap();

        let value = callback.call::<_, LuaValue>(args).unwrap();

        if let Some(s) = value_to_string(value) {
            compilation_data.src += &s;
        }
    }
}

impl MacroProvider for Rc<RefCell<LuaMacroProvider>> {
    fn is_macro_id(&self, id: &str) -> bool {
        self.borrow().is_macro_id(id)
    }

    fn handle_macro(
        &self,
        ctx: LuaContext,
        id: &str,
        compilation_data: &mut compiler::CompilationData,
        macro_invocation: compiler::MacroInvocation,
    ) {
        self.borrow()
            .handle_macro(ctx, id, compilation_data, macro_invocation);
    }
}

thread_local! {
    static LUA_MACRO_PROVIDER: RefCell<Option<Rc<RefCell<LuaMacroProvider>>>> = RefCell::new(None);
}

pub fn get_threadlocal_macro_provider() -> Rc<RefCell<LuaMacroProvider>> {
    LUA_MACRO_PROVIDER.with(|macro_provider| {
        let mut macro_provider = macro_provider.borrow_mut();

        if macro_provider.is_none() {
            let macro_provider_new = LuaMacroProvider {
                registered_macros: Default::default(),
            };

            macro_provider.replace(Rc::new(RefCell::new(macro_provider_new)));
        }

        Rc::clone(macro_provider.as_ref().unwrap())
    })
}

pub fn get_register_luafn(ctx: LuaContext) -> LuaFunction {
    let func = ctx
        .create_function::<_, (), _>(|ctx, (id, callback): (String, LuaFunction)| {
            let lua_macro_provider = get_threadlocal_macro_provider();

            {
                let mut lua_macro_provider = lua_macro_provider.borrow_mut();
                lua_macro_provider.register_macro(ctx, &id, callback);
            }

            Ok(())
        })
        .unwrap();

    func
}
