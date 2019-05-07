use indexmap::IndexMap;
use std::fmt::Write;
use std::fs;
use std::path::{Path};

use failure::{err_msg, format_err, Error, Fail, ResultExt};
use matches::matches;
use pest::iterators::{Pair};
use pest::Parser;
use rlua::Lua;

use ceres_parsers::lua;

use crate::context::CeresContext;
use crate::util;

#[derive(Debug, Fail)]
pub enum PreprocessorError {
    #[fail(display = "Parsing error in {}", _1)]
    ParseError(#[fail(cause)] pest::error::Error<lua::Rule>, String),
    // #[fail(display = "Macro syntax error in {}, {}", _0, _1)]
    // GenericError(String)
}

pub struct CodeUnit {
    source: String,
}

impl CodeUnit {
    pub fn source(&self) -> &str {
        &self.source
    }
}

pub struct CodeProcessor<'a> {
    lua: &'a mut Lua,
    context: &'a CeresContext,
    code_units: IndexMap<String, CodeUnit>,
}

impl<'a> CodeProcessor<'a> {
    pub fn new(lua: &'a mut Lua, context: &'a CeresContext) -> CodeProcessor<'a> {
        CodeProcessor {
            lua,
            context,
            code_units: Default::default(),
        }
    }

    pub fn code_units(&self) -> impl Iterator<Item = (&String, &CodeUnit)> {
        self.code_units.iter()
    }

    pub fn add_file<P: AsRef<Path>>(
        &mut self,
        module_name: &str,
        file_path: P,
    ) -> Result<(), Error> {
        if !self.code_units.contains_key(module_name) {
            let path_string = file_path.as_ref().to_str().unwrap().to_string();

            let source = fs::read_to_string(&file_path).with_context(|e| {
                format!(
                    "Preprocessor: could not add file {}, reason: {}",
                    path_string, e
                )
            })?;
            let source = self.preprocess_file(&source, file_path)?;

            let code_unit = CodeUnit { source };

            self.code_units.insert(module_name.to_string(), code_unit);
        }

        Ok(())
    }

    fn preprocess_file<P: AsRef<Path>>(
        &mut self,
        input: &str,
        file_path: P,
    ) -> Result<String, Error> {
        let parsed = lua::LuaParser::parse(lua::Rule::Chunk, input).map_err(|err| {
            PreprocessorError::ParseError(err, file_path.as_ref().to_str().unwrap().to_string())
        })?;
        let mut out_string = String::new();
        let mut emitted_index = 0;

        parsed
            .flatten()
            .filter(|pair| matches!(pair.as_rule(), lua::Rule::MacroCall))
            .try_for_each(|pair| {
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
                } else {
                    Ok(())
                }
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
            .context
            .src_file_path(format!("{}.lua", string_content.replace(".", "/")))?;

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

        let full_include_path = self.context.file_path(string_content);

        if !full_include_path.is_file() {
            return Err(format_err!(
                "{} is not a valid file",
                full_include_path.display()
            ));
        }

        let include_content = fs::read_to_string(full_include_path)?;

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
                let values: rlua::MultiValue =
                    ctx.load(&format!("return ({})", exp.as_str())).eval()?;

                for value in values {
                    match value {
                        rlua::Value::Function(func) => {
                            let values = func.call::<_, rlua::MultiValue>(())?;

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

    fn emit_lua_value(&self, value: rlua::Value, out_string: &mut String) {
        match value {
            rlua::Value::String(string) => {
                write!(out_string, r#""{}""#, string.to_str().unwrap()).unwrap();
            }
            rlua::Value::Number(number) => {
                write!(out_string, "{}", &number.to_string()).unwrap();
            }
            rlua::Value::Integer(number) => {
                write!(out_string, "{}", &number.to_string()).unwrap();
            }

            _ => {}
        }
    }
}
