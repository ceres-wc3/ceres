#![allow(dead_code)]

extern crate fs_extra as fsx;


mod compiler;
pub mod context;
mod util;
use failure::{AsFail, Error, Fail, ResultExt};
use rlua::prelude::*;
use std::fmt::Write;
use std::fs;
use std::fs::File;

use std::sync::Arc;

use compiler::CodeCompiler;
use std::path::{Path, PathBuf};

use std::error::Error as StdError;


// #[derive(Fail)]
// pub enum CeresError {

// }

/// Main orchestrator of Ceres.
///
/// This type is responsible for managing and orchestrating the map build and run process.
/// It will try to find an existing Ceres project in the current working directory when created.
///
/// It uses a [`CeresContext`] to source information about the user's Ceres config and to get
/// file paths required by the build process.
///
/// [`CeresContext`]: ./context/struct.CeresContext.html
// pub struct Ceres {
//     lua: Lua,
//     context: context::CeresContext,
// }

// impl Ceres {
//     /// Creates a new Ceres orchestrator.
//     ///
//     /// Will fail if it cannot find the user's config, located in `.ceres/config.toml` in the
//     /// user's home directory, or if it cannot open/read the current working directory.
//     pub fn new() -> Result<Ceres, Error> {
//         let lua = Lua::new();
//         let root_dir = std::env::current_dir()?;

//         let context = context::CeresContext::new(&root_dir)?;

//         Ok(Ceres { lua, context })
//     }

//     /// Builds and runs the specified map in WC3, using the user's Ceres config to determine
//     /// how to run the map.
//     ///
//     /// `map_name` is the map directory name, not path. When looking for the map, it will look
//     /// for `./maps/<map_name>` to find the map.
//     ///
//     /// This is equivalent to calling `.build_map()` and then running the game manually with the
//     /// `-loadfile` param.
//     pub fn run_map(&mut self, map_name: &str) -> Result<(), Error> {
//         self.build_map(map_name).context("Could not build map.")?;

//         let config = self.context.config();
//         let wc3_cmd = config.run.wc3_start_command.clone();
//         let map_launch_dir = if config.run.is_wine.unwrap_or(false) {
//             format!(
//                 "{}{}",
//                 config.run.wine_disk_prefix.as_ref().unwrap(),
//                 self.context.map_target_dir_path(map_name).display()
//             )
//         } else {
//             self.context
//                 .map_target_dir_path(map_name)
//                 .display()
//                 .to_string()
//         };

//         let mut cmd = Command::new(wc3_cmd);

//         let window_mode = config
//             .run
//             .window_mode
//             .as_ref()
//             .map_or("windowedfullscreen", |s| &s);

//         let log_file = File::create(self.context.file_path("war3.log"))
//             .context("Could not create wc3 log file.")?;

//         cmd.arg("-loadfile")
//             .arg(map_launch_dir)
//             .arg("-windowmode")
//             .arg(window_mode)
//             .stdout(log_file.try_clone()?)
//             .stderr(log_file.try_clone()?);

//         println!("starting wc3 with command line: {:?}", cmd);
//         cmd.spawn().context("Could not launch wc3.")?;

//         Ok(())
//     }

//     /// Builds the specified map, which involves:
//     ///     1. Copying the map folder to `./target/`
//     ///     2. Building the map script, processing macros and includes.
//     ///     3. Writing out the map script to `./target/<map_name>/war3map.lua`
//     ///
//     /// The resulting map artifact will be located in `./target/<map_name>`
//     ///
//     /// `map_name` is the map directory name, not path. When looking for the map, it will look
//     /// for `./maps/<map_name>` to find the map.
//     pub fn build_map(&mut self, map_name: &str) -> Result<(), Error> {
//         let map_source_dir = self.context.map_src_dir_path(map_name)?;
//         let map_target_dir = self.context.map_target_dir_path(map_name);

//         println!(
//             "building map {}, source dir {}, target dir {}",
//             map_name,
//             map_source_dir.display(),
//             map_target_dir.display()
//         );

//         let mut copy_options = fsx::dir::CopyOptions::new();
//         copy_options.overwrite = true;
//         copy_options.copy_inside = false;
//         fsx::dir::copy(
//             &map_source_dir,
//             &self.context.target_dir_path(),
//             &copy_options,
//         )
//         .context("Could not create target folder.")?;

//         let script = self
//             .build_script(map_name)
//             .context("Could not build the script.")?;

//         let warcraft_script_path = map_target_dir.join("war3map.lua");
//         fsx::file::write_all(warcraft_script_path, &script)
//             .context("Could not write war3map.lua to target.")?;

//         println!("finished building map {}", map_name);

//         Ok(())
//     }

//     fn build_script(&mut self, map_name: &str) -> Result<String, Error> {
//         const HEADER: &str = include_str!("resource/ceres_header.lua");
//         const POST: &str = include_str!("resource/ceres_post.lua");

//         println!("building map script for {}", map_name);

//         let map_script = fs::read_to_string(self.context.map_file_path(map_name, "war3map.lua")?)
//             .context("Could not read map's war3map.lua")?;
//         let mut main_script: String = String::new();

//         writeln!(main_script, "{}", HEADER).unwrap();
//         writeln!(main_script, "{}", map_script).unwrap();

//         let mut preprocessor = CodeCompiler::new(&mut self.lua, &self.context);
//         preprocessor.add_file("main", self.context.src_file_path("main.lua")?)?;

//         for (module_number, (module_name, module_source)) in preprocessor.code_units().enumerate() {
//             writeln!(
//                 main_script,
//                 "local function __module_{}()\n    {}\nend",
//                 module_number,
//                 module_source.source().replace("\n", "\n    ")
//             )
//             .unwrap();

//             writeln!(
//                 main_script,
//                 r#"__modules["{}"] = {{initialized = false, cached = nil, loader = __module_{}}}"#,
//                 module_name, module_number
//             )
//             .unwrap();
//         }

//         writeln!(main_script).unwrap();
//         writeln!(main_script, "{}", POST).unwrap();

//         Ok(main_script)
//     }
// }

use std::rc::Rc;

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

struct CeresBuildArgs {
    mode: CeresBuildMode,
    src_folders: Vec<PathBuf>,
    main_path: PathBuf,
}

impl CeresBuildArgs {
    fn new(table: rlua::Table) -> Result<CeresBuildArgs, Error> {
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
            mode: build_mode,
            src_folders: table
                .get::<&str, rlua::Table>("src_folders")?
                .sequence_values::<String>()
                .filter_map(|val| val.map(PathBuf::from).ok())
                .collect(),
            main_path: table.get::<&str, String>("main_path")?.into(),
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
        self.cause.downcast_ref::<failure::Compat<Error>>().map(|compat| compat.get_ref())
    }
}

impl Fail for ArcError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.as_error().map(Error::as_fail).and_then(Fail::cause)
    }
}

struct Ceres2 {
    lua: Rc<Lua>,
    project_dir: PathBuf,
}

impl Ceres2 {
    fn new(lua: Rc<Lua>, project_dir: PathBuf) -> Ceres2 {
        Ceres2 { lua, project_dir }
    }

    fn start_build(&self, ctx: rlua::Context, args: rlua::Table) -> Result<(), Error> {
        let build_options = CeresBuildArgs::new(args)?;

        match &build_options.mode {
            CeresBuildMode::Script {
                map_script_path,
                script_out_path,
            } => {
                let result = self.build_script(
                    &map_script_path,
                    &build_options.main_path,
                    &build_options.src_folders,
                )?;

                println!("{}", result);
            }

            CeresBuildMode::Map { map_path, out_path } => unimplemented!(),
        }


        Ok(())
    }

    fn build_script(
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

        for (module_number, (module_name, module_source)) in preprocessor.code_units().enumerate() {
            writeln!(
                main_script,
                "local function __module_{}()\n    {}\nend",
                module_number,
                module_source.source().replace("\n", "\n    ")
            )
            .unwrap();

            writeln!(
                main_script,
                r#"__modules["{}"] = {{initialized = false, cached = nil, loader = __module_{}}}"#,
                module_name, module_number
            )
            .unwrap();
        }

        writeln!(main_script).unwrap();
        writeln!(main_script, "{}", POST).unwrap();

        Ok(main_script)
    }
}

pub fn run_build_script(
    build_script_path: Option<PathBuf>,
    script_args: Vec<&str>,
) -> Result<(), Error> {
    let build_script_path =
        build_script_path.unwrap_or_else(|| std::env::current_dir().unwrap().join("build.lua"));
    let build_script = fs::read_to_string(&build_script_path)
        .with_context(|_| format!("error reading {:?}", build_script_path))?;

    let lua = Rc::new(Lua::new());
    // TODO: add --project-dir option support
    let ceres = Ceres2::new(Rc::clone(&lua), std::env::current_dir().unwrap());

    let result: Result<(), Error> = lua.context(|ctx| {
        // scoped so that we don't have to synchronize anything...
        ctx.scope(|scope| {
            let globals = ctx.globals();

            let build_fn = scope
                .create_function(
                    |ctx, args: rlua::Table| match ceres.start_build(ctx, args) {
                        Ok(_) => Ok(()),
                        Err(err) => Err(rlua::Error::external(err)),
                    },
                )
                .unwrap();

            globals.set("ARGS", script_args).unwrap();
            globals.set("build", build_fn).unwrap();

            let result: Result<_, rlua::Error> =
                ctx.load(&build_script).set_name("build.lua")?.exec();

            result.map_err::<Error, _>(|cause| match cause {
                rlua::Error::CallbackError { cause, .. } => match &*cause {
                    rlua::Error::ExternalError(cause) => {
                        ArcError { cause: Arc::clone(cause) }.into()
                    }
                    other => other.clone().into(),
                },
                other => other.into(),
            })?;

            Ok(())
        })
    });

    result.context("failed to run build script")?;

    Ok(())
}