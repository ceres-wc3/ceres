use std::path::{PathBuf, Path};
use std::path::Component;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;

use pest::iterators::Pair;
use pest::Parser;
use rlua::prelude::*;
use indexmap::IndexMap;
use walkdir::WalkDir;
use itertools::Itertools;

use ceres_parsers::lua;

use crate::lua::util::evaluate_macro_args;
use crate::lua::util::lvalue_to_str;
use crate::error::*;

pub trait ModuleProvider {
    fn module_src(&self, module_name: &str) -> Option<String>;

    fn module_path(&self, module_name: &str) -> Option<PathBuf>;
}

pub struct ProjectModuleProvider {
    directories: Vec<PathBuf>,

    known_modules: HashMap<String, PathBuf>,
}

impl ProjectModuleProvider {
    pub fn new(directories: &[PathBuf]) -> ProjectModuleProvider {
        ProjectModuleProvider {
            directories: directories.into(),

            known_modules: Default::default(),
        }
    }

    pub fn scan(&mut self) {
        for dir in &self.directories {
            Self::scan_dir(&mut self.known_modules, &dir);
        }
    }

    fn scan_dir<P: AsRef<Path>>(modules: &mut HashMap<String, PathBuf>, path: P) {
        let path = path.as_ref();
        for entry in WalkDir::new(path).follow_links(true) {
            let entry = entry.unwrap();

            let ext = entry.path().extension();
            if ext.is_some() && ext.unwrap() == "lua" {
                let relative_path = entry.path().strip_prefix(path).unwrap();
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

                modules.insert(module_path.into(), entry.into_path());
            }
        }
    }
}

impl ModuleProvider for ProjectModuleProvider {
    fn module_src(&self, module_name: &str) -> Option<String> {
        let path = self.known_modules.get(module_name);

        path.and_then(|s| fs::read_to_string(s).ok())
    }

    fn module_path(&self, module_name: &str) -> Option<PathBuf> {
        self.known_modules.get(module_name).cloned()
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
    ) -> Result<(), MacroInvocationError>;
}

#[derive(Debug)]
pub struct CompilationData {
    pub(crate) name: String,
    pub(crate) src:  String,
}

#[derive(Debug)]
pub struct CompiledModule {
    pub(crate) name: String,
    pub(crate) src:  String,
}

#[derive(Debug)]
pub struct MacroInvocation<'src> {
    pub(crate) id:         &'src str,
    pub(crate) args:       Pair<'src, lua::Rule>,
    pub(crate) span_start: usize,
    pub(crate) span_end:   usize,
}

pub struct ScriptCompiler<'lua, MO: ModuleProvider, MA: MacroProvider> {
    pub(crate) ctx: LuaContext<'lua>,

    map_script: Option<String>,

    // map of modules that have already been compiled
    compiled_modules:  IndexMap<String, CompiledModule>,
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

            map_script: None,

            compiled_modules: Default::default(),
            compiling_modules: Default::default(),

            module_provider,
            macro_provider,
        }
    }

    pub fn emit_script(&self) -> String {
        const SCRIPT_HEADER: &str = include_str!("resource/map_header.lua");
        const SCRIPT_FOOTER: &str = include_str!("resource/map_footer.lua");

        let mut out = String::new();

        out += SCRIPT_HEADER.trim();
        out += "\n\n";

        if let Some(map_script) = &self.map_script {
            out += "--[[ map script start ]]\n";
            out += map_script.trim();
            out += "\n--[[ map script end ]]\n\n";
        }

        for (id, compiled_module) in self.compiled_modules.iter() {
            let module_header_comment = format!("--[[ start of module \"{}\" ]]\n", id);
            let module_header = format!(
                r#"ceres.modules["{name}"] = {{initialized = false, cached = nil, source = [================["#,
                name = id
            );
            let module_source = format!("\n{}\n", compiled_module.src);
            let module_footer = "]================]}\n";
            let module_footer_comment = format!("--[[ end of module \"{}\" ]]\n\n", id);

            out += &module_header_comment;
            out += &module_header;
            out += &module_source;
            out += &module_footer;
            out += &module_footer_comment;
        }

        out += SCRIPT_FOOTER.trim();
        out += "\n";

        out
    }

    /// tries to find and compile the given module by it's module name
    /// using the ModuleProvider
    pub fn add_module(&mut self, module_name: &str, optional: bool) -> Result<(), CompilerError> {
        if self.compiling_modules.contains(module_name) {
            return Err(CompilerError::CyclicalDependency {
                module_name: module_name.into(),
            });
        }

        if self.compiled_modules.contains_key(module_name) {
            return Ok(());
        }

        let src = self.module_provider.module_src(module_name);

        if src.is_none() {
            if optional {
                return Ok(());
            } else {
                return Err(CompilerError::ModuleNotFound {
                    module_name: module_name.into(),
                });
            }
        }

        let src = src.unwrap();

        self.compiling_modules.insert(module_name.into());
        let compiled_module = self.compile_module(module_name, &src);

        if let Err(error) = compiled_module {
            match &error {
                CompilerError::ModuleError { .. } => return Err(error),
                _ => {
                    return Err(CompilerError::ModuleError {
                        module_name: module_name.into(),
                        module_path: self.module_provider.module_path(module_name).unwrap(),
                        error:       Box::new(error),
                    })
                }
            }
        }

        let compiled_module = compiled_module.unwrap();

        self.compiling_modules.remove(module_name);
        self.compiled_modules
            .insert(module_name.into(), compiled_module);

        Ok(())
    }

    pub fn set_map_script(&mut self, map_script: String) {
        self.map_script = Some(map_script);
    }

    /// will compile a single module with the given module name and source,
    /// as well as all of it's transitive dependencies, while processing macros
    fn compile_module(
        &mut self,
        module_name: &str,
        src: &str,
    ) -> Result<CompiledModule, CompilerError> {
        let parsed = lua::LuaParser::parse(lua::Rule::Chunk, src)?;

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

            if let Some(invocation) = self.macro_invocation(pair.clone()) {
                next_pair_start = invocation.span_end;

                compilation_data.src += &src[emitted_index..invocation.span_start];
                emitted_index = invocation.span_end;

                self.handle_macro(src, &mut compilation_data, invocation)
                    .map_err(|err| match err {
                        MacroInvocationError::CompilerError { error } => error,
                        _ => CompilerError::MacroError {
                            error:      Box::new(err),
                            diagnostic: pest::error::Error::new_from_span(
                                pest::error::ErrorVariant::CustomError { message: "".into() },
                                pair.as_span(),
                            ),
                        },
                    })?;
            }
        }

        if emitted_index < src.len() {
            compilation_data.src += &src[emitted_index..src.len()];
        }

        Ok(CompiledModule {
            name: compilation_data.name,
            src:  compilation_data.src,
        })
    }

    fn is_macro_id(&self, id: &str) -> bool {
        match id {
            "compiletime" | "require" => true,
            id => self.macro_provider.is_macro_id(id),
        }
    }

    fn handle_macro(
        &mut self,
        src: &str,
        compilation_data: &mut CompilationData,
        macro_invocation: MacroInvocation,
    ) -> Result<(), MacroInvocationError> {
        let id = macro_invocation.id;
        let newline_count = src[macro_invocation.span_start..macro_invocation.span_end]
            .chars()
            .filter(|c| *c == '\n')
            .count();

        match id {
            "require" => self.handle_macro_require(compilation_data, macro_invocation)?,
            "compiletime" => self.handle_macro_compiletime(compilation_data, macro_invocation)?,
            id => self.macro_provider.handle_macro(
                self.ctx,
                id,
                compilation_data,
                macro_invocation,
            )?,
        }

        compilation_data.src += &("\n").repeat(newline_count);

        Ok(())
    }

    fn handle_macro_require(
        &mut self,
        compilation_data: &mut CompilationData,
        macro_invocation: MacroInvocation,
    ) -> Result<(), MacroInvocationError> {
        let args = evaluate_macro_args(self.ctx, macro_invocation.args)?.into_vec();

        if args.is_empty() {
            return Err(MacroInvocationError::message(
                "Require macro requires at least one argument".into(),
            ));
        }

        let optional = if args.len() >= 2 {
            if let LuaValue::Boolean(optional) = &args[1] {
                *optional
            } else {
                false
            }
        } else {
            false
        };

        if let LuaValue::String(module_name) = &args[0] {
            let module_name = module_name.to_str().unwrap();
            if optional {
                compilation_data.src += &format!("require(\"{}\", true)", module_name)
            } else {
                compilation_data.src += &format!("require(\"{}\")", module_name);
            }
            self.add_module(module_name, optional)?;
        } else {
            return Err(MacroInvocationError::message(
                "Require macro's first argument must be a string".into(),
            ));
        }

        Ok(())
    }

    fn handle_macro_compiletime(
        &mut self,
        compilation_data: &mut CompilationData,
        macro_invocation: MacroInvocation,
    ) -> Result<(), MacroInvocationError> {
        let mut args = evaluate_macro_args(self.ctx, macro_invocation.args)
            .unwrap()
            .into_vec();

        if args.len() > 1 || args.is_empty() {
            return Err(MacroInvocationError::message(
                "Compiletime macro must have exactly one argument".into(),
            ));
        }

        let arg = args.remove(0);

        let value = if let LuaValue::Function(func) = arg {
            func.call::<_, LuaValue>(())?
        } else {
            arg
        };

        if let Some(s) = lvalue_to_str(value) {
            compilation_data.src += &s;
        }

        Ok(())
    }

    /// Will try to extract a macro invocation out of the given pair, returning `None` if it can't find one.
    fn macro_invocation<'src>(&self, pair: Pair<'src, lua::Rule>) -> Option<MacroInvocation<'src>> {
        if pair.as_rule() != lua::Rule::FunctionCall {
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

        let ident = var.into_iter().next()?.into_inner().next()?;

        if ident.as_rule() != lua::Rule::Ident {
            return None;
        }

        let id = ident.as_str();

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
