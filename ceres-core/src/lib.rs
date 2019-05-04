extern crate fs_extra as fsx;

pub mod context;
mod processor;
mod util;

use failure::{Error, ResultExt};
use log::info;
use rlua::prelude::*;
use std::fmt::Write;
use std::fs;
use std::fs::File;
use std::process::Command;


use crate::processor::CodeProcessor;

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
pub struct Ceres {
    lua: Lua,
    context: context::CeresContext,
}

impl Ceres {
    /// Creates a new Ceres orchestrator.
    ///
    /// Will fail if it cannot find the user's config, located in `.ceres/config.toml` in the
    /// user's home directory, or if it cannot open/read the current working directory.
    pub fn new() -> Result<Ceres, Error> {
        let lua = Lua::new();
        let root_dir = std::env::current_dir()?;

        let context = context::CeresContext::new(&root_dir)?;

        Ok(Ceres { lua, context })
    }

    /// Builds and runs the specified map in WC3, using the user's Ceres config to determine
    /// how to run the map.
    ///
    /// `map_name` is the map directory name, not path. When looking for the map, it will look
    /// for `./maps/<map_name>` to find the map.
    ///
    /// This is equivalent to calling `.build_map()` and then running the game manually with the
    /// `-loadfile` param.
    pub fn run_map(&mut self, map_name: &str) -> Result<(), Error> {
        self.build_map(map_name).context("Could not build map.")?;

        let config = self.context.config();
        let wc3_cmd = config.run.wc3_start_command.clone();
        let map_launch_dir = if config.run.is_wine.unwrap_or(false) {
            format!(
                "{}{}",
                config.run.wine_disk_prefix.as_ref().unwrap(),
                self.context.map_target_dir_path(map_name).display()
            )
        } else {
            self.context
                .map_target_dir_path(map_name)
                .display()
                .to_string()
        };

        let mut cmd = Command::new(wc3_cmd);

        let window_mode = config
            .run
            .window_mode
            .as_ref()
            .map_or("windowedfullscreen", |s| &s);

        let log_file = File::create(self.context.file_path("war3.log"))
            .context("Could not create wc3 log file.")?;

        cmd.arg("-loadfile")
            .arg(map_launch_dir)
            .arg("-windowmode")
            .arg(window_mode)
            .stdout(log_file.try_clone()?)
            .stderr(log_file.try_clone()?);

        info!("starting wc3 with command line: {:?}", cmd);
        cmd.spawn().context("Could not launch wc3.")?;

        Ok(())
    }

    /// Builds the specified map, which involves:
    ///     1. Copying the map folder to `./target/`
    ///     2. Building the map script, processing macros and includes.
    ///     3. Writing out the map script to `./target/<map_name>/war3map.lua`
    ///
    /// The resulting map artifact will be located in `./target/<map_name>`
    ///
    /// `map_name` is the map directory name, not path. When looking for the map, it will look
    /// for `./maps/<map_name>` to find the map.
    pub fn build_map(&mut self, map_name: &str) -> Result<(), Error> {
        let map_source_dir = self.context.map_src_dir_path(map_name)?;
        let map_target_dir = self.context.map_target_dir_path(map_name);

        info!(
            "building map {}, source dir {}, target dir {}",
            map_name,
            map_source_dir.display(),
            map_target_dir.display()
        );

        let mut copy_options = fsx::dir::CopyOptions::new();
        copy_options.overwrite = true;
        copy_options.copy_inside = false;
        fsx::dir::copy(
            &map_source_dir,
            &self.context.target_dir_path(),
            &copy_options,
        )
        .context("Could not create target folder.")?;

        let script = self
            .build_script(map_name)
            .context("Could not build the script.")?;

        let warcraft_script_path = map_target_dir.join("war3map.lua");
        fsx::file::write_all(warcraft_script_path, &script)
            .context("Could not write war3map.lua to target.")?;

        info!("finished building map {}", map_name);

        Ok(())
    }

    fn build_script(&mut self, map_name: &str) -> Result<String, Error> {
        const HEADER: &str = include_str!("resource/ceres_header.lua");
        const POST: &str = include_str!("resource/ceres_post.lua");

        info!("building map script for {}", map_name);

        let map_script = fs::read_to_string(self.context.map_file_path(map_name, "war3map.lua")?)
            .context("Could not read map's war3map.lua")?;
        let mut main_script: String = String::new();

        writeln!(main_script, "{}", HEADER).unwrap();
        writeln!(main_script, "{}", map_script).unwrap();

        let mut preprocessor = CodeProcessor::new(&mut self.lua, &self.context);
        preprocessor.add_file("main", self.context.src_file_path("main.lua")?)?;

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
