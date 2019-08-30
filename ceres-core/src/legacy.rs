pub fn run_map(&mut self, path: &PathBuf) -> Result<(), Error> {
    let config = config::CeresConfig::new(&self.project_dir)?;

    if let Some(run_config) = config.run {
        let wc3_cmd = run_config.wc3_start_command.clone();

        let map_launch_dir: PathBuf = if run_config.is_wine.unwrap_or(false) {
            format!(
                "{}{}",
                run_config
                    .wine_disk_prefix
                    .ok_or_else(|| err_msg("missing wine_disk_prefix key from config"))?,
                path.canonicalize().unwrap().display()
            )
            .into()
        } else {
            path.into()
        };

        let mut cmd = Command::new(wc3_cmd);

        let window_mode = run_config
            .window_mode
            .unwrap_or_else(|| "windowedfullscreen".into());

        let log_file = fs::File::create(self.project_dir.join("war3.log"))
            .context("Could not create wc3 log file.")?;

        cmd.arg("-loadfile")
            .arg(map_launch_dir)
            .arg("-windowmode")
            .arg(window_mode)
            .stdout(log_file.try_clone()?)
            .stderr(log_file.try_clone()?);

        println!("starting wc3 with command line: {:?}", cmd);
        cmd.spawn().context("Could not launch wc3.")?;
    } else {
        return Err(err_msg("missing [run] section from config"));
    }

    Ok(())
}