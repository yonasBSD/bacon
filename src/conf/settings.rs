use {
    crate::*,
    anyhow::{
        Result,
        bail,
    },
    std::{
        collections::HashMap,
        path::PathBuf,
    },
};

/// The settings used in the application.
///
/// They're immutable during the execution of the missions.
#[derive(Debug, Clone)]
pub struct Settings {
    pub additional_alias_args: Option<Vec<String>>,
    pub additional_job_args: Vec<String>,
    pub all_features: bool,
    pub arg_job: Option<ConcreteJobRef>,
    /// Path of the files which were used to build the settings
    /// (note that not all settings come from files)
    pub config_files: Vec<PathBuf>,
    pub default_job: ConcreteJobRef,
    pub exports: ExportsSettings,
    pub features: Option<String>, // comma separated list
    pub help_line: bool,
    pub jobs: HashMap<String, Job>,
    pub keybindings: KeyBindings,
    pub no_default_features: bool,
    pub reverse: bool,
    pub summary: bool,
    pub wrap: bool,
    /// Whether to listen for actions on a unix socket (if on unix)
    pub listen: bool,
    pub all_jobs: Job,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            arg_job: Default::default(),
            additional_job_args: Default::default(),
            additional_alias_args: Default::default(),
            summary: false,
            wrap: true,
            reverse: false,
            help_line: true,
            no_default_features: Default::default(),
            all_features: Default::default(),
            features: Default::default(),
            keybindings: Default::default(),
            jobs: Default::default(),
            default_job: Default::default(),
            exports: Default::default(),
            config_files: Default::default(),
            listen: false,
            all_jobs: Default::default(),
        }
    }
}

impl Settings {
    /// Read the settings from all configuration files and arguments.
    ///
    ///
    /// Hardcoded defaults are overridden by the following configuration elements, in order:
    /// * the default `bacon.toml` file (embedded in the binary)
    /// * the global `prefs.toml`, from user config directory
    /// * the file whose path is in environment variable `BACON_PREFS`
    /// * the workspace.metadata.bacon config in the workspace level `Cargo.toml` file
    /// * the workspace level `bacon.toml` file in workspace-root/bacon.toml
    /// * the workspace level `bacon.toml` file in workspace-root/.config/.bacon.toml
    /// * the package.metadata.bacon config in the package level `Cargo.toml` file
    /// * the package level `bacon.toml` file in package-root/.bacon.toml
    /// * the package level `bacon.toml` file in package-root/.config/.bacon.toml
    /// * the file whose path is in environment variable `BACON_CONFIG`
    /// * the content of the `--config-toml` argument
    /// * args given as arguments, coming from the cli call
    pub fn read(
        args: &Args,
        context: &Context,
    ) -> Result<Self> {
        let mut settings = Settings::default();

        let default_package_config = Config::default_package_config();
        settings.apply_config(&default_package_config);

        let paths = vec![
            bacon_prefs_path(),
            config_path_from_env("BACON_PREFS"),
            context.workspace_cargo_path(),
            context.workspace_config_path(),
            context.workspace_dot_config_path(),
            Some(context.package_cargo_path()),
            Some(context.package_config_path()),
            Some(context.package_dot_config_path()),
            config_path_from_env("BACON_CONFIG"),
        ];
        for path in paths.into_iter().flatten() {
            if path.exists() {
                let configs = Config::from_path_detect(&path)?;
                if !configs.is_empty() {
                    info!("config loaded from {path:?}");
                    settings.register_config_file(path.clone());
                    for config in configs {
                        settings.apply_config(&config);
                    }
                }
            }
        }

        if let Some(toml) = &args.config_toml {
            let config = toml::from_str(toml)?;
            info!("config loaded from --config-toml: {:#?}", &config);
            settings.apply_config(&config);
        }

        settings.apply_args(args);
        settings.check()?;
        info!("settings: {:#?}", &settings);
        Ok(settings)
    }

    pub fn register_config_file(
        &mut self,
        path: PathBuf,
    ) {
        self.config_files.push(path);
    }

    /// Apply one of the configuration elements, overriding
    /// defaults and previously applied configuration elements
    pub fn apply_config(
        &mut self,
        config: &Config,
    ) {
        self.all_jobs.apply(&config.all_jobs);
        if let Some(b) = config.summary {
            self.summary = b;
        }
        if let Some(b) = config.wrap {
            self.wrap = b;
        }
        if let Some(b) = config.reverse {
            self.reverse = b;
        }
        if let Some(b) = config.help_line {
            self.help_line = b;
        }
        #[allow(deprecated)] // for compatibility
        if config.vim_keys == Some(true) {
            self.keybindings.add_vim_keys();
        }
        if let Some(keybindings) = config.keybindings.as_ref() {
            self.keybindings.add_all(keybindings);
        }
        if config.additional_alias_args.is_some() {
            self.additional_alias_args
                .clone_from(&config.additional_alias_args);
        }
        for (name, job) in &config.jobs {
            self.jobs.insert(name.clone(), job.clone());
        }
        if let Some(default_job) = &config.default_job {
            self.default_job = default_job.clone();
        }
        if let Some(listen) = config.listen {
            self.listen = listen;
        }
        self.exports.apply_config(config);
    }
    pub fn apply_args(
        &mut self,
        args: &Args,
    ) {
        if let Some(job) = &args.job {
            self.arg_job = Some(job.clone());
        }
        if args.no_summary {
            self.summary = false;
        }
        if args.summary {
            self.summary = true;
        }
        if args.no_wrap {
            self.wrap = false;
        }
        if args.wrap {
            self.wrap = true;
        }
        if args.no_reverse {
            self.reverse = false;
        }
        if args.help_line {
            self.help_line = true;
        }
        if args.no_help_line {
            self.help_line = false;
        }
        if args.export_locations {
            self.exports.set_locations_export_auto(true);
        }
        if args.no_export_locations {
            self.exports.set_locations_export_auto(false);
        }
        if args.reverse {
            self.reverse = true;
        }
        if args.no_default_features {
            self.no_default_features = true;
        }
        if args.all_features {
            self.all_features = true;
        }
        if args.features.is_some() {
            self.features.clone_from(&args.features);
        }
        #[cfg(unix)]
        {
            if args.listen {
                self.listen = true;
            }
            if args.no_listen {
                self.listen = false;
            }
        }
        self.additional_job_args
            .clone_from(&args.additional_job_args);
    }

    pub fn check(&self) -> Result<()> {
        if self.jobs.is_empty() {
            bail!("Invalid configuration : no job found");
        }
        if let NameOrAlias::Name(name) = &self.default_job.name_or_alias {
            if !self.jobs.contains_key(name) {
                bail!("Invalid configuration : default job ({name:?}) not found in jobs");
            }
        }
        Ok(())
    }
}
