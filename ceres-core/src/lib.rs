#![allow(dead_code)]

mod config;
mod compiler;
mod util;

use failure::{Error, Fail, ResultExt};
use rlua::prelude::*;
use matches::matches;

use std::error::Error as StdError;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use compiler::CodeCompiler;

struct CeresBuildArtifact {
    path: PathBuf,
    kind: CeresBuildArtifactKind,
}

impl CeresBuildArtifact {
    fn new(kind: CeresBuildArtifactKind, path: PathBuf) -> CeresBuildArtifact {
        CeresBuildArtifact { kind, path }
    }
}

enum CeresBuildArtifactKind {
    Script,
    MapFolder,
    MapArchive,
}

enum CeresBuildMode {
    Script {
        map_script_path: PathBuf,
        script_out_path: PathBuf,
    },
    Map {
        map_path: PathBuf,
        out_path: PathBuf,
    },
}

pub enum CeresRunMode {
    Build,
    RunMap,
    LiveReload,
}

struct CeresBuildArgs {
    mode:        CeresBuildMode,
    src_folders: Vec<PathBuf>,
    main_path:   PathBuf,
}

impl CeresBuildArgs {
    fn new(table: LuaTable) -> Result<CeresBuildArgs, Error> {
        let is_script_only = table.get::<&str, bool>("script_only").unwrap();

        let build_mode = if is_script_only {
            CeresBuildMode::Script {
                map_script_path: table.get::<&str, String>("map_script_path")?.into(),
                script_out_path: table.get::<&str, String>("script_out_path")?.into(),
            }
        } else {
            CeresBuildMode::Map {
                map_path: table.get::<&str, String>("map_path")?.into(),
                out_path: table.get::<&str, String>("out_path")?.into(),
            }
        };

        let build_options = CeresBuildArgs {
            mode:        build_mode,
            src_folders: table
                .get::<&str, LuaTable>("src_folders")?
                .sequence_values::<String>()
                .filter_map(|val| val.map(PathBuf::from).ok())
                .collect(),
            main_path:   table.get::<&str, String>("main_path")?.into(),
        };

        Ok(build_options)
    }
}

#[derive(Debug)]
struct ArcError {
    cause: Arc<dyn StdError + Sync + Send + 'static>,
}

impl std::fmt::Display for ArcError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(fmt, "{}", self.cause)
    }
}

impl ArcError {
    fn as_error(&self) -> Option<&Error> {
        self.cause
            .downcast_ref::<failure::Compat<Error>>()
            .map(failure::Compat::get_ref)
    }
}

impl Fail for ArcError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.as_error().map(Error::as_fail).and_then(Fail::cause)
    }
}

struct Ceres {
    lua:             Rc<Lua>,
    project_dir:     PathBuf,
    built_artifacts: Vec<CeresBuildArtifact>,
}

impl Ceres {
    fn new(lua: Rc<Lua>, project_dir: PathBuf) -> Ceres {
        Ceres {
            lua,
            project_dir,
            built_artifacts: Default::default(),
        }
    }

    fn artifacts(&self) -> &[CeresBuildArtifact] {
        &self.built_artifacts
    }

    fn start_build(&mut self, args: LuaTable) -> Result<(), Error> {
        let build_options = CeresBuildArgs::new(args)?;

        let artifact = match &build_options.mode {
            CeresBuildMode::Script {
                map_script_path,
                script_out_path,
            } => {
                let path = self.build_script(
                    map_script_path,
                    &build_options.main_path,
                    &build_options.src_folders,
                    script_out_path,
                )?;

                CeresBuildArtifact::new(CeresBuildArtifactKind::Script, path)
            }

            CeresBuildMode::Map { map_path, out_path } => {
                let path = self.build_map(
                    map_path,
                    out_path,
                    &build_options.main_path,
                    &build_options.src_folders,
                )?;

                CeresBuildArtifact::new(CeresBuildArtifactKind::MapFolder, path)
            }
        };

        self.built_artifacts.push(artifact);

        Ok(())
    }

    fn build_map(
        &self,
        map_path: &PathBuf,
        out_path: &PathBuf,
        main_path: &PathBuf,
        src_folders: &[PathBuf],
    ) -> Result<PathBuf, Error> {
        let map_script_path = map_path.join("war3map.lua");
        let script_out_path = out_path.join("war3map.lua");

        util::copy_dir_from_to(map_path, out_path)
            .with_context(|_| format!("could not prepare target folder {:?}", out_path))?;

        self.build_script(&map_script_path, main_path, src_folders, &script_out_path)?;

        Ok(out_path.into())
    }

    fn build_script(
        &self,
        map_script_path: &PathBuf,
        main_path: &PathBuf,
        src_folders: &[PathBuf],
        script_out_path: &PathBuf,
    ) -> Result<PathBuf, Error> {
        let script = self.compile_script(&map_script_path, &main_path, &src_folders)?;

        fs::write(script_out_path, script).with_context(|_| {
            format!(
                "could not write out compilation result to {:?}",
                script_out_path
            )
        })?;

        Ok(script_out_path.into())
    }

    fn compile_script(
        &self,
        map_script_path: &PathBuf,
        main_path: &PathBuf,
        src_folders: &[PathBuf],
    ) -> Result<String, Error> {
        const HEADER: &str = include_str!("resource/ceres_header.lua");
        const POST: &str = include_str!("resource/ceres_post.lua");

        let map_script =
            fs::read_to_string(&map_script_path).context("could not read map script")?;
        let mut main_script: String = String::new();

        writeln!(main_script, "{}", HEADER).unwrap();
        writeln!(main_script, "{}", map_script).unwrap();

        let mut preprocessor = CodeCompiler::new(&self.lua, src_folders, &self.project_dir);
        preprocessor.add_file("main", main_path)?;

        for (module_name, module_source) in preprocessor.code_units() {
            let module_header_comment = format!("--[[ start of module \"{}\" ]]\n", module_name);
            let module_header = format!(
                r#"__modules["{name}"] = {{initialized = false, cached = nil, loader = function()"#,
                name = module_name
            );
            let module_source = format!(
                "\n    {}\n",
                module_source.source().replace("\n", "\n    ").trim()
            );
            let module_footer = "end}\n";
            let module_footer_comment = format!("--[[ end of module \"{}\" ]]\n\n", module_name);

            main_script += &module_header_comment;
            main_script += &module_header;
            main_script += &module_source;
            main_script += &module_footer;
            main_script += &module_footer_comment;
        }

        writeln!(main_script).unwrap();
        writeln!(main_script, "{}", POST).unwrap();

        Ok(main_script)
    }

    pub fn run_map(&mut self, path: &PathBuf) -> Result<(), Error> {
        unimplemented!()
        // self.build_map(map_name).context("Could not build map.")?;

        // let config = self.context.config();
        // let wc3_cmd = config.run.wc3_start_command.clone();
        // let map_launch_dir = if config.run.is_wine.unwrap_or(false) {
        //     format!(
        //         "{}{}",
        //         config.run.wine_disk_prefix.as_ref().unwrap(),
        //         self.context.map_target_dir_path(map_name).display()
        //     )
        // } else {
        //     self.context
        //         .map_target_dir_path(map_name)
        //         .display()
        //         .to_string()
        // };

        // let mut cmd = Command::new(wc3_cmd);

        // let window_mode = config
        //     .run
        //     .window_mode
        //     .as_ref()
        //     .map_or("windowedfullscreen", |s| &s);

        // let log_file = File::create(self.context.file_path("war3.log"))
        //     .context("Could not create wc3 log file.")?;

        // cmd.arg("-loadfile")
        //     .arg(map_launch_dir)
        //     .arg("-windowmode")
        //     .arg(window_mode)
        //     .stdout(log_file.try_clone()?)
        //     .stderr(log_file.try_clone()?);

        // println!("starting wc3 with command line: {:?}", cmd);
        // cmd.spawn().context("Could not launch wc3.")?;

        // Ok(())
    }
}

pub fn execute(
    run_mode: CeresRunMode,
    project_dir: PathBuf,
    script_args: Vec<&str>,
) -> Result<(), Error> {
    let build_script_path = project_dir.join("build.lua");
    let build_script = fs::read_to_string(&build_script_path)
        .with_context(|_| format!("error reading {:?}", build_script_path))?;

    let lua = Rc::new(Lua::new());
    let mut ceres = Ceres::new(Rc::clone(&lua), project_dir);

    let result: Result<(), Error> = lua.context(|ctx| {
        // scoped so that we don't have to synchronize anything...
        ctx.scope(|scope| {
            let globals = ctx.globals();

            let build_fn = scope
                .create_function_mut(|ctx, args: LuaTable| match ceres.start_build(args) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(LuaError::external(err)),
                })
                .unwrap();

            globals.set("ARGS", script_args).unwrap();
            globals.set("build", build_fn).unwrap();

            let result: Result<_, LuaError> = ctx.load(&build_script).set_name("build.lua")?.exec();

            result.map_err::<Error, _>(|cause| match cause {
                LuaError::CallbackError { cause, .. } => match &*cause {
                    LuaError::ExternalError(cause) => ArcError {
                        cause: Arc::clone(cause),
                    }
                    .into(),

                    other => other.clone().into(),
                },
                other => other.into(),
            })?;

            Ok(())
        })
    });

    result.context("failed to run build script")?;

    if let CeresRunMode::RunMap = run_mode {
        println!("Trying to run the built map...");

        let artifacts = ceres.artifacts();

        let map_artifacts: Vec<_> = artifacts
            .iter()
            .filter(|x| {
                matches!(x.kind, CeresBuildArtifactKind::MapArchive)
                    || matches!(x.kind, CeresBuildArtifactKind::MapFolder)
            })
            .collect();

        if map_artifacts.len() > 1 {
            println!("[ERROR] The build script produced more than one map artifact - unclear which map to run.");
        } else if map_artifacts.is_empty() {
            println!("[ERROR] The build script produced no map artifacts. Nothing to run.")
        } else {
            let map_artifact = map_artifacts[0];
            println!("Choosing {:?} as the map to run.", &map_artifact.path);

            let path = map_artifact.path.clone();

            ceres.run_map(&path)?;
        }
    }

    Ok(())
}
