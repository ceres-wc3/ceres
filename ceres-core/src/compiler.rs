use indexmap::IndexMap;

#[derive(Debug)]
pub struct CompiledModule {
    pub(crate) name: String,
    pub(crate) src:  String,
}

pub struct ScriptCompiler {
    compiled_modules: IndexMap<String, CompiledModule>,
}

impl ScriptCompiler {
    pub fn emit_script(&self) -> String {
        // Uncomment this loop to get 100% CPU usage and persistent memory leak...

        for (id, compiled_module) in &self.compiled_modules {
        }

        unimplemented!()
    }
}
