use indexmap::IndexMap;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use failure::{err_msg, format_err, Error, Fail};
use matches::matches;
use pest::iterators::Pair;
use pest::Parser;
use rlua::prelude::*;

use ceres_parsers::lua;

use crate::util;

#[derive(Debug, Fail)]
pub enum CompilerError {
    #[fail(display = "Parsing error in {:?}", path)]
    ParseError {
        #[fail(cause)]
        cause: pest::error::Error<lua::Rule>,
        path: PathBuf,
    },
    #[fail(display = "Cannot access file {:?}", path)]
    IOError {
        #[fail(cause)]
        cause: std::io::Error,
        path: PathBuf,
    },
    #[fail(display = "Error in {:?}", path)]
    CodeError {
        #[fail(cause)]
        cause: CodeError,
        path: PathBuf,
    },
}

#[derive(Debug, Fail)]
#[fail(display = "{}", diagnostic)]
pub struct CodeError {
    diagnostic: pest::error::Error<lua::Rule>,
    #[fail(cause)]
    cause: Error,
}

pub struct CodeUnit {
    source: String,
}

impl CodeUnit {
    pub fn source(&self) -> &str {
        &self.source
    }
}

pub struct CodeCompiler<'a> {
    lua:        &'a Lua,
    code_units: IndexMap<String, CodeUnit>,
    src_dirs:   &'a [PathBuf],
    root_dir:   &'a PathBuf,
}

impl<'a> CodeCompiler<'a> {
    pub fn new(lua: &'a Lua, src_dirs: &'a [PathBuf], root_dir: &'a PathBuf) -> CodeCompiler<'a> {
        CodeCompiler {
            lua,
            code_units: Default::default(),
            src_dirs,
            root_dir,
        }
    }

    fn find_src_file<S>(&self, name: S) -> Option<PathBuf>
    where
        S: AsRef<Path>,
    {
        for folder in self.src_dirs {
            let file_path = folder.join(name.as_ref());

            if file_path.is_file() {
                return Some(file_path);
            }
        }

        None
    }

    pub fn code_units(&self) -> impl Iterator<Item = (&String, &CodeUnit)> {
        self.code_units.iter()
    }

    pub fn add_file<S>(&mut self, module_name: &str, file_path: S) -> Result<(), Error>
    where
        S: AsRef<Path>,
    {
        if !self.code_units.contains_key(module_name) {
            let source = fs::read_to_string(&file_path).map_err(|err| CompilerError::IOError {
                cause: err,
                path:  file_path.as_ref().to_path_buf(),
            })?;

            let source = self.compile_file(&source, file_path)?;

            let code_unit = CodeUnit { source };

            self.code_units.insert(module_name.to_string(), code_unit);
        }

        Ok(())
    }

    fn compile_file<S>(&mut self, input: &str, file_path: S) -> Result<String, CompilerError>
    where
        S: AsRef<Path>,
    {
        let parsed = lua::LuaParser::parse(lua::Rule::Chunk, input).map_err(|err| {
            CompilerError::ParseError {
                cause: err,
                path:  file_path.as_ref().to_path_buf(),
            }
        })?;

        let mut out_string = String::new();
        let mut emitted_index = 0;

        parsed
            .flatten()
            .filter(|pair| matches!(pair.as_rule(), lua::Rule::MacroCall))
            .try_for_each::<_, Result<(), CompilerError>>(|pair| {
                let span_start = pair.as_span().start();
                let span_end = pair.as_span().end();

                // emit everything up to the macro invocation
                if span_start > emitted_index {
                    write!(out_string, "{}", &input[emitted_index..span_start]).unwrap();
                }

                emitted_index = span_end;

                let macro_name = util::find_pairs_with_rule(&pair, lua::Rule::Macro)
                    .next()
                    .unwrap()
                    .as_str();

                let macro_args = util::find_pairs_with_rule(&pair, lua::Rule::ExpList).next();

                if macro_args.is_some() {
                    self.process_macro(macro_name, macro_args.unwrap(), &mut out_string)
                        .map_err(|cause| {
                            let pest_error = pest::error::ErrorVariant::CustomError::<lua::Rule> {
                                message: cause.to_string(),
                            };
                            let diagnostic =
                                pest::error::Error::new_from_span(pest_error, pair.as_span());
                            CompilerError::CodeError {
                                cause: CodeError { diagnostic, cause },
                                path:  file_path.as_ref().to_path_buf(),
                            }
                        })?;
                }

                Ok(())
            })?;

        // if we have anything left-over, emit it
        if emitted_index < input.len() {
            write!(out_string, "{}", &input[emitted_index..input.len()]).unwrap();
        }

        Ok(out_string)
    }

    fn process_macro(
        &mut self,
        macro_name: &str,
        macro_args: Pair<lua::Rule>,
        out_string: &mut String,
    ) -> Result<(), Error> {
        match macro_name {
            "require" => self.process_require(macro_args, out_string)?,
            "include" => self.process_include(macro_args, out_string)?,
            "compiletime" => self.process_compiletime(macro_args, out_string)?,
            _ => {}
        }

        Ok(())
    }

    fn process_require(
        &mut self,
        macro_args: Pair<lua::Rule>,
        out_string: &mut String,
    ) -> Result<(), Error> {
        let exp = macro_args.into_inner().next().unwrap();
        let string_literal = util::find_pairs_with_rule(&exp, lua::Rule::LiteralString)
            .next()
            .ok_or_else(|| err_msg("require macro must take a string literal"))?;

        let string_content = string_literal
            .into_inner()
            .next()
            .ok_or_else(|| err_msg("require macro must have content"))?
            .as_str();

        let full_require_path = self
            .find_src_file(format!("{}.lua", string_content.replace(".", "/")))
            .ok_or_else(|| format_err!("cannot find module {}", string_content))?;

        self.add_file(string_content, full_require_path)?;

        write!(out_string, r#"require("{}")"#, string_content).unwrap();

        Ok(())
    }

    fn process_include(
        &mut self,
        macro_args: Pair<lua::Rule>,
        out_string: &mut String,
    ) -> Result<(), Error> {
        let exp = macro_args.into_inner().next().unwrap();
        let string_literal = util::find_pairs_with_rule(&exp, lua::Rule::LiteralString)
            .next()
            .ok_or_else(|| err_msg("include macro must take a string literal"))?;

        let string_content = string_literal
            .into_inner()
            .next()
            .ok_or_else(|| err_msg("include macro must have content"))?
            .as_str();

        let full_include_path = self.root_dir.join(string_content);

        let include_content =
            fs::read_to_string(&full_include_path).map_err(|err| CompilerError::IOError {
                cause: err,
                path:  full_include_path.to_path_buf(),
            })?;

        write!(out_string, "{}", include_content).unwrap();

        Ok(())
    }

    fn process_compiletime(
        &self,
        macro_args: Pair<lua::Rule>,
        out_string: &mut String,
    ) -> Result<(), Error> {
        for exp in macro_args.into_inner() {
            let result: Result<(), Error> = self.lua.context(|ctx| {
                let values: LuaMultiValue =
                    ctx.load(&format!("return ({})", exp.as_str())).eval()?;

                for value in values {
                    match value {
                        LuaValue::Function(func) => {
                            let values = func.call::<_, LuaMultiValue>(())?;

                            for value in values {
                                self.emit_lua_value(value, out_string);
                            }
                        }

                        _ => self.emit_lua_value(value, out_string),
                    }
                }

                Ok(())
            });

            result?;
        }

        Ok(())
    }

    fn emit_lua_value(&self, value: LuaValue, out_string: &mut String) {
        match value {
            LuaValue::String(string) => {
                write!(out_string, r#""{}""#, string.to_str().unwrap()).unwrap();
            }
            LuaValue::Number(number) => {
                write!(out_string, "{}", &number.to_string()).unwrap();
            }
            LuaValue::Integer(number) => {
                write!(out_string, "{}", &number.to_string()).unwrap();
            }

            _ => {}
        }
    }
}
