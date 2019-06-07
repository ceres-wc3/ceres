use std::path::PathBuf;
use std::path::Path;
use std::path::Component;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;

use pest::iterators::Pair;
use pest::Parser;
use rlua::prelude::*;
use err_derive::Error;
use indexmap::IndexMap;
use walkdir::WalkDir;
use itertools::Itertools;

use ceres_parsers::lua;

use crate::lua::util::evaluate_macro_args;
use crate::lua::util::value_to_string;

enum CompilerError {
    ModuleNotFound {},
    ParserFailed {},
}

pub trait ModuleProvider {
    fn module_src(&self, module_name: &str) -> Option<String>;
}

pub struct ProjectModuleProvider {
    src_dir: PathBuf,
    lib_dir: PathBuf,

    known_modules: HashMap<String, PathBuf>,
}

impl ProjectModuleProvider {
    pub fn new(src_dir: PathBuf, lib_dir: PathBuf) -> ProjectModuleProvider {
        ProjectModuleProvider {
            src_dir,
            lib_dir,

            known_modules: Default::default(),
        }
    }

    pub fn scan(&mut self) {
        for entry in WalkDir::new(&self.src_dir).follow_links(true) {
            let entry = entry.unwrap();

            let ext = entry.path().extension();
            if ext.is_some() && ext.unwrap() == "lua" {
                let relative_path = entry.path().strip_prefix(&self.src_dir).unwrap();
                let module_path = relative_path
                    .components()
                    .filter_map(|s| {
                        if let Component::Normal(s) = s {
                            s.to_str()
                        } else {
                            None
                        }
                    })
                    .join(".");

                let module_path = &module_path[..(module_path.len() - 4)];

                self.known_modules.insert(module_path.into(), entry.into_path());
            }
        }
    }
}

impl ModuleProvider for ProjectModuleProvider {
    fn module_src(&self, module_name: &str) -> Option<String> {
        let path = self.known_modules.get(module_name);

        path.and_then(|s| {
            fs::read_to_string(s).ok()
        })
    }
}

pub trait MacroProvider {
    fn is_macro_id(&self, id: &str) -> bool;

    fn handle_macro(
        &self,
        ctx: LuaContext,
        id: &str,
        compilation_data: &mut CompilationData,
        macro_invocation: MacroInvocation,
    );
}

pub struct CompilationData {
    pub(crate) name: String,
    pub(crate) src:  String,
}

pub struct CompiledModule {
    pub(crate) name: String,
    pub(crate) src:  String,
}

pub struct MacroInvocation<'src> {
    pub(crate) id:         &'src str,
    pub(crate) args:       Pair<'src, lua::Rule>,
    pub(crate) span_start: usize,
    pub(crate) span_end:   usize,
}

pub struct ScriptCompiler<'lua, MO: ModuleProvider, MA: MacroProvider> {
    pub(crate) ctx: LuaContext<'lua>,

    // map of modules that have already been compiled
    compiled_modules: IndexMap<String, CompiledModule>,
    // set of modules that are currently in compilation
    compiling_modules: HashSet<String>,

    module_provider: MO,
    macro_provider:  MA,
}

impl<'lua, MO: ModuleProvider, MA: MacroProvider> ScriptCompiler<'lua, MO, MA> {
    pub fn new(
        ctx: LuaContext<'lua>,
        module_provider: MO,
        macro_provider: MA,
    ) -> ScriptCompiler<'lua, MO, MA> {
        ScriptCompiler {
            ctx,

            compiled_modules: Default::default(),
            compiling_modules: Default::default(),

            module_provider,
            macro_provider,
        }
    }

    pub fn emit_script(&self) -> String {
        const SCRIPT_HEADER: &str = include_str!("resource/ceres_header.lua");
        const SCRIPT_POST: &str = include_str!("resource/ceres_post.lua");

        let mut out = String::new();

        out += SCRIPT_HEADER;

        for (id, compiled_module) in &self.compiled_modules {
            let module_header_comment = format!("--[[ start of module \"{}\" ]]\n", id);
            let module_header = format!(
                r#"__modules["{name}"] = {{initialized = false, cached = nil, loader = function()"#,
                name = id
            );
            let module_source = format!(
                "\n    {}\n",
                compiled_module.src.replace("\n", "\n    ").trim()
            );
            let module_footer = "end}\n";
            let module_footer_comment = format!("--[[ end of module \"{}\" ]]\n\n", id);

            out += &module_header_comment;
            out += &module_header;
            out += &module_source;
            out += &module_footer;
            out += &module_footer_comment;
        }

        out += "\n";
        out += SCRIPT_POST;
        out += "\n";

        out
    }

    /// tries to find and compile the given module by it's module name
    /// using the ModuleProvider
    pub fn add_module(&mut self, module_name: &str) {
        if self.compiling_modules.contains(module_name) {
            // TODO: Error Handling
            // Error case: A cyclic dependency
            unimplemented!("unhandled error: a cyclic dependency detected");
        }

        if self.compiled_modules.contains_key(module_name) {
            // happy path, module is already compiled
            // we don't need to do anything
            return;
        }

        let src = self.module_provider.module_src(module_name);

        if src.is_none() {
            // TODO: Error Handling
            // Error case: Module does not exist
            // This is not an error if the module is required as optional
            unimplemented!("unhandled error: module {} does not exist", module_name)
        }

        let src = src.unwrap();

        self.compiling_modules.insert(module_name.into());
        let compiled_module = self.compile_module(module_name, &src);
        self.compiling_modules.remove(module_name);
        self.compiled_modules
            .insert(module_name.into(), compiled_module);
    }

    /// will compile a single module with the given module name and source,
    /// as well as all of it's transitive dependencies, while processing macros
    fn compile_module(&mut self, module_name: &str, src: &str) -> CompiledModule {
        let parsed = lua::LuaParser::parse(lua::Rule::Chunk, src);

        if parsed.is_err() {
            // TODO: Error handling
            // Error case: unparseable lua file
            unimplemented!("unhandled error: lua file cannot be parsed")
        }

        let parsed = parsed.unwrap();

        let mut compilation_data = CompilationData {
            name: module_name.into(),
            src:  String::new(),
        };

        let mut next_pair_start = 0;
        let mut emitted_index = 0;
        for pair in parsed.flatten() {
            // ignore any pairs that are inside a macro invocation
            if pair.as_span().start() < next_pair_start {
                continue;
            }

            if let Some(invocation) = self.macro_invocation(pair) {
                next_pair_start = invocation.span_end;

                compilation_data.src += &src[emitted_index..invocation.span_start];
                emitted_index = invocation.span_end;

                self.handle_macro(&mut compilation_data, invocation);
            }
        }

        if emitted_index < src.len() {
            compilation_data.src += &src[emitted_index..src.len()];
        }

        CompiledModule {
            name: compilation_data.name,
            src:  compilation_data.src,
        }
    }

    fn is_macro_id(&self, id: &str) -> bool {
        match id {
            "include" | "compiletime" | "require" => true,
            id => self.macro_provider.is_macro_id(id),
        }
    }

    fn handle_macro(
        &mut self,
        compilation_data: &mut CompilationData,
        macro_invocation: MacroInvocation,
    ) {
        let id = macro_invocation.id;

        match id {
            "require" => self.handle_macro_require(compilation_data, macro_invocation),
            "compiletime" => self.handle_macro_compiletime(compilation_data, macro_invocation),
            id => {
                self.macro_provider
                    .handle_macro(self.ctx, id, compilation_data, macro_invocation)
            }
        }
    }

    fn handle_macro_require(
        &mut self,
        compilation_data: &mut CompilationData,
        macro_invocation: MacroInvocation,
    ) {
        let args = evaluate_macro_args(self.ctx, macro_invocation.args)
            .unwrap()
            .into_vec();

        if args.is_empty() {
            // TODO: Error handling
            // Error case: Require must have at least one argument

            unimplemented!("unhandled error: require macro requires at least one argument");
        }

        if let LuaValue::String(module_name) = &args[0] {
            let module_name = module_name.to_str().unwrap();
            self.add_module(module_name);
        } else {
            // TODO: Error handling
            // Error case: The first argument must be a string

            unimplemented!("unhandled error: require macro's first argument must be a string")
        }
    }

    fn handle_macro_compiletime(
        &mut self,
        compilation_data: &mut CompilationData,
        macro_invocation: MacroInvocation,
    ) {
        let mut args = evaluate_macro_args(self.ctx, macro_invocation.args)
            .unwrap()
            .into_vec();

        if args.len() > 1 || args.is_empty() {
            // TODO: Error handling
            // Error case: Compiletime macro supports only a single argument

            unimplemented!("unhandled error: compiletime macro must have exactly one argument")
        }

        let arg = args.remove(0);

        let value = if let LuaValue::Function(func) = arg {
            // TODO: Error handling
            // Error case: Compiletime callback errored out

            func.call::<_, LuaValue>(()).unwrap()
        } else {
            arg
        };

        if let Some(s) = value_to_string(value) {
            compilation_data.src += &s;
        }
    }

    /// Will try to extract a macro invocation out of the given pair, returning `None` if it can't find one.
    fn macro_invocation<'src>(&self, pair: Pair<'src, lua::Rule>) -> Option<MacroInvocation<'src>> {
        if pair.as_rule() != lua::Rule::Value {
            return None;
        }

        let var = pair
            .clone()
            .into_inner()
            .find(|i| i.as_rule() == lua::Rule::Var)?;

        // we want the var to consist only of a single ident
        // if it's anything more complex, then it's never a macro
        // i really wish i had a proper AST here

        let var = var.into_inner().collect::<Vec<_>>();

        if var.len() > 1 {
            return None;
        }

        let atomic_exp = var.into_iter().next()?.into_inner().next()?;

        if atomic_exp.as_rule() != lua::Rule::Ident {
            return None;
        }

        let id = atomic_exp.as_str();

        if !self.is_macro_id(id) {
            return None;
        }

        let calls = pair
            .clone()
            .into_inner()
            .filter(|i| i.as_rule() == lua::Rule::Call)
            .collect::<Vec<_>>();

        if calls.is_empty() {
            return None;
        }

        let call = calls.into_iter().next()?;
        let span_start = pair.as_span().start();
        let span_end = call.as_span().end();

        let simple_call = call
            .into_inner()
            .find(|i| i.as_rule() == lua::Rule::SimpleCall)?;

        let args = simple_call.into_inner().next()?;

        Some(MacroInvocation {
            id,
            args,
            span_start,
            span_end,
        })
    }
}
