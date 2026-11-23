use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde::Deserialize;

/// Command-line interface for the media catalog generator.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "media-catalog",
    version,
    about = "Generate and publish an incremental catalog for a YYYY/MM media archive"
)]
pub struct Cli {
    /// Root of the read-only media archive (organized as YYYY/MM).
    #[arg(long, value_name = "PATH")]
    pub media_root: Option<PathBuf>,

    /// Root for published catalog data. Contains `catalog/` and `thumbs/`.
    #[arg(long, value_name = "PATH")]
    pub data_root: Option<PathBuf>,

    /// Root for rebuildable build state. Contains `state.sqlite` and `build.lock`.
    #[arg(long, value_name = "PATH")]
    pub state_root: Option<PathBuf>,

    /// URL prefix used for original media links.
    #[arg(long, value_name = "PATH")]
    pub media_url_prefix: Option<String>,

    /// URL prefix used for thumbnail links.
    #[arg(long, value_name = "PATH")]
    pub thumb_url_prefix: Option<String>,

    /// Number of parallel thumbnail workers.
    #[arg(long, value_name = "N")]
    pub jobs: Option<usize>,

    /// Regenerate thumbnails even when the source fingerprint is unchanged.
    #[arg(long)]
    pub force: bool,

    /// Delete thumbnail files that no longer belong to a catalog item.
    #[arg(long)]
    pub prune: bool,

    /// Scan and report planned work without writing output or state.
    #[arg(long)]
    pub dry_run: bool,

    /// Suppress progress reporting on stderr.
    #[arg(long)]
    pub quiet: bool,

    /// Allow pruning and state cleanup after a sharp item-count drop.
    #[arg(long)]
    pub allow_mount_drop: bool,

    /// Optional TOML configuration file. Defaults to ./media-catalog.toml when present.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Optional authored events content file. Overrides `events_file` from the
    /// configuration file.
    #[arg(long, value_name = "PATH")]
    pub events: Option<PathBuf>,

    /// Republish the catalog from state without scanning, probing, or thumbnails.
    #[arg(long)]
    pub events_only: bool,

    /// Publish geocoordinates from EXIF and video location tags.
    #[arg(long)]
    pub include_location: bool,
}

/// Optional TOML configuration shared by both binaries. Values here are
/// overridden by explicit CLI flags; keys owned by the other binary are parsed
/// and ignored.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FileConfig {
    pub media_root: Option<PathBuf>,
    pub data_root: Option<PathBuf>,
    pub state_root: Option<PathBuf>,
    pub media_url_prefix: Option<String>,
    pub thumb_url_prefix: Option<String>,
    pub jobs: Option<usize>,
    pub force: Option<bool>,
    pub prune: Option<bool>,
    pub dry_run: Option<bool>,
    pub quiet: Option<bool>,
    pub allow_mount_drop: Option<bool>,
    pub events_file: Option<PathBuf>,
    pub events_only: Option<bool>,
    /// Publish geocoordinates. Ignored by the runtime server.
    pub include_location: Option<bool>,
    /// Socket address for the runtime server. Ignored by the generator.
    pub bind: Option<String>,
    /// Maximum concurrent server media streams. Ignored by the generator.
    pub max_streams: Option<usize>,
}

impl FileConfig {
    /// Load the shared configuration file. An explicit path must exist;
    /// otherwise `./media-catalog.toml` is used when present.
    pub fn load(explicit: Option<&Path>) -> Result<Self> {
        let path = match explicit {
            Some(path) => {
                if !path.exists() {
                    bail!("configuration file not found: {}", path.display());
                }
                Some(path.to_path_buf())
            }
            None => {
                let candidate = PathBuf::from("media-catalog.toml");
                candidate.exists().then_some(candidate)
            }
        };

        let Some(path) = path else {
            return Ok(Self::default());
        };

        let text = fs::read_to_string(&path)
            .with_context(|| format!("reading configuration file {}", path.display()))?;
        toml::from_str(&text)
            .with_context(|| format!("parsing configuration file {}", path.display()))
    }
}

/// Fully resolved and validated configuration.
#[derive(Debug, Clone)]
pub struct Config {
    pub media_root: PathBuf,
    pub data_root: PathBuf,
    pub state_root: PathBuf,
    pub state_db: PathBuf,
    pub media_url_prefix: String,
    pub thumb_url_prefix: String,
    pub jobs: usize,
    pub force: bool,
    pub prune: bool,
    pub dry_run: bool,
    pub quiet: bool,
    pub allow_mount_drop: bool,
    /// Authored events file, or `None` when the catalog has no events.
    pub events_file: Option<PathBuf>,
    pub events_only: bool,
    pub include_location: bool,
}

impl Config {
    /// Resolve CLI flags over an optional TOML file and validate the result.
    pub fn load(cli: Cli) -> Result<Self> {
        let file_config = FileConfig::load(cli.config.as_deref())?;
        let cores = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1);
        Self::resolve(cli, file_config, cores)
    }

    /// Build a configuration without reading a TOML file. Useful for tests and embedding.
    pub fn with_roots(
        media_root: impl Into<PathBuf>,
        data_root: impl Into<PathBuf>,
        state_root: impl Into<PathBuf>,
    ) -> Self {
        let state_root = state_root.into();
        let state_db = state_db_for(&state_root);
        Self {
            media_root: media_root.into(),
            data_root: data_root.into(),
            state_root,
            state_db,
            media_url_prefix: "/media".to_string(),
            thumb_url_prefix: "/thumbs".to_string(),
            jobs: 1,
            force: false,
            prune: false,
            dry_run: false,
            quiet: true,
            allow_mount_drop: false,
            events_file: None,
            events_only: false,
            include_location: false,
        }
    }

    /// Resolve configuration sources. CLI values win over TOML values; missing required
    /// roots are an error.
    pub fn resolve(cli: Cli, file: FileConfig, cores: usize) -> Result<Self> {
        let cwd = std::env::current_dir().context("reading the current directory")?;

        let media_root = cli.media_root.or(file.media_root).ok_or_else(|| {
            anyhow::anyhow!("--media-root is required (or set media_root in TOML)")
        })?;
        let data_root = cli
            .data_root
            .or(file.data_root)
            .ok_or_else(|| anyhow::anyhow!("--data-root is required (or set data_root in TOML)"))?;
        let state_root = cli.state_root.or(file.state_root).ok_or_else(|| {
            anyhow::anyhow!("--state-root is required (or set state_root in TOML)")
        })?;

        let media_root = absolutize(&cwd, &media_root);
        let data_root = absolutize(&cwd, &data_root);
        let state_root = absolutize(&cwd, &state_root);
        let state_db = state_db_for(&state_root);

        let media_url_prefix = normalize_prefix(
            cli.media_url_prefix
                .or(file.media_url_prefix)
                .unwrap_or_else(|| "/media".to_string()),
            "--media-url-prefix",
        )?;
        let thumb_url_prefix = normalize_prefix(
            cli.thumb_url_prefix
                .or(file.thumb_url_prefix)
                .unwrap_or_else(|| "/thumbs".to_string()),
            "--thumb-url-prefix",
        )?;

        let jobs = cli.jobs.or(file.jobs).unwrap_or_else(|| cores.min(4));
        let force = cli.force || file.force.unwrap_or(false);
        let prune = cli.prune || file.prune.unwrap_or(false);
        let dry_run = cli.dry_run || file.dry_run.unwrap_or(false);
        let quiet = cli.quiet || file.quiet.unwrap_or(false);
        let allow_mount_drop = cli.allow_mount_drop || file.allow_mount_drop.unwrap_or(false);
        let events_only = cli.events_only || file.events_only.unwrap_or(false);
        let include_location = cli.include_location || file.include_location.unwrap_or(false);

        let events_file =
            resolve_events_path(cli.events.as_deref().or(file.events_file.as_deref()), &cwd)?;

        let config = Self {
            media_root,
            data_root,
            state_root,
            state_db,
            media_url_prefix,
            thumb_url_prefix,
            jobs,
            force,
            prune,
            dry_run,
            quiet,
            allow_mount_drop,
            events_file,
            events_only,
            include_location,
        };
        config.validate()?;
        Ok(config)
    }

    /// Validate paths, prefixes, and the relationships between the storage roots.
    pub fn validate(&self) -> Result<()> {
        if self.jobs == 0 {
            bail!("--jobs must be at least 1");
        }
        if !self.events_only {
            if !self.media_root.exists() {
                bail!("media root does not exist: {}", self.media_root.display());
            }
            if !self.media_root.is_dir() {
                bail!(
                    "media root is not a directory: {}",
                    self.media_root.display()
                );
            }
        }
        if self.data_root.exists() && !self.data_root.is_dir() {
            bail!("data root is not a directory: {}", self.data_root.display());
        }
        if self.state_root.exists() && !self.state_root.is_dir() {
            bail!(
                "state root is not a directory: {}",
                self.state_root.display()
            );
        }

        for (left, left_name, right, right_name) in [
            (&self.media_root, "media", &self.data_root, "data"),
            (&self.media_root, "media", &self.state_root, "state"),
            (&self.data_root, "data", &self.state_root, "state"),
        ] {
            if roots_overlap(left, right) {
                bail!(
                    "{left_name} root and {right_name} root must be separate directories: {} and {}",
                    left.display(),
                    right.display()
                );
            }
        }

        Ok(())
    }

    pub fn media_root(&self) -> &Path {
        &self.media_root
    }

    pub fn data_root(&self) -> &Path {
        &self.data_root
    }

    pub fn state_root(&self) -> &Path {
        &self.state_root
    }

    pub fn state_db(&self) -> &Path {
        &self.state_db
    }
}

/// Build state lives directly in the state root.
fn state_db_for(state_root: &Path) -> PathBuf {
    state_root.join("state.sqlite")
}

/// Resolve the events content file. When no path is configured the catalog has
/// no events; a configured path must exist and be a file.
fn resolve_events_path(configured: Option<&Path>, cwd: &Path) -> Result<Option<PathBuf>> {
    let Some(path) = configured else {
        return Ok(None);
    };
    let path = absolutize(cwd, path);
    if !path.exists() {
        bail!("events file not found: {}", path.display());
    }
    if !path.is_file() {
        bail!("events path is not a file: {}", path.display());
    }
    Ok(Some(path))
}

fn absolutize(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        normalize_lexically(path)
    } else {
        normalize_lexically(&cwd.join(path))
    }
}

/// Remove `.` components and resolve `..` lexically without touching the filesystem.
pub fn normalize_lexically(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    result.push(component.as_os_str());
                }
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

fn path_contains(path: &Path, root: &Path) -> bool {
    path.starts_with(root) && path != root
}

fn roots_overlap(left: &Path, right: &Path) -> bool {
    left == right || path_contains(left, right) || path_contains(right, left)
}

/// Normalize a URL prefix to a leading-slash, trailing-slash-free path.
pub fn normalize_prefix(value: String, flag: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("{flag} must not be empty");
    }
    if trimmed.contains('?') || trimmed.contains('#') {
        bail!("{flag} must be a path prefix without a query or fragment");
    }
    let mut prefix = trimmed.trim_end_matches('/').to_string();
    if !prefix.starts_with('/') {
        prefix.insert(0, '/');
    }
    if prefix == "/" {
        bail!("{flag} must not be '/' because it would collide with the web tree root");
    }
    if prefix
        .split('/')
        .skip(1)
        .any(|segment| segment.is_empty() || segment == ".." || segment == ".")
    {
        bail!("{flag} must not contain empty, '.' or '..' segments");
    }
    Ok(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli_with_roots(media: &str, data: &str, state: &str) -> Cli {
        Cli {
            media_root: Some(PathBuf::from(media)),
            data_root: Some(PathBuf::from(data)),
            state_root: Some(PathBuf::from(state)),
            ..Cli::parse_from(["media-catalog"])
        }
    }

    #[test]
    fn cli_overrides_toml() {
        let temp = tempfile::tempdir().unwrap();
        let toml_media = temp.path().join("toml-media");
        let toml_data = temp.path().join("toml-data");
        let toml_state = temp.path().join("toml-state");
        let cli_media = temp.path().join("cli-media");
        let cli_data = temp.path().join("cli-data");
        let cli_state = temp.path().join("cli-state");
        std::fs::create_dir_all(&toml_media).unwrap();
        std::fs::create_dir_all(&cli_media).unwrap();

        let file = FileConfig {
            media_root: Some(toml_media),
            data_root: Some(toml_data),
            state_root: Some(toml_state),
            jobs: Some(9),
            media_url_prefix: Some("/old-media".to_string()),
            ..FileConfig::default()
        };
        let cli = Cli {
            media_root: Some(cli_media.clone()),
            data_root: Some(cli_data.clone()),
            state_root: Some(cli_state.clone()),
            jobs: Some(2),
            media_url_prefix: Some("/new-media".to_string()),
            ..Cli::parse_from(["media-catalog"])
        };
        let config = Config::resolve(cli, file, 8).expect("resolve");
        assert_eq!(config.media_root, cli_media);
        assert_eq!(config.data_root, cli_data);
        assert_eq!(config.state_root, cli_state);
        assert_eq!(config.state_db, cli_state.join("state.sqlite"));
        assert_eq!(config.jobs, 2);
        assert_eq!(config.media_url_prefix, "/new-media");
    }

    #[test]
    fn missing_roots_are_rejected() {
        let cli = Cli::parse_from(["media-catalog"]);
        let error = Config::resolve(cli, FileConfig::default(), 4).unwrap_err();
        assert!(error.to_string().contains("--media-root"));
    }

    #[test]
    fn missing_data_root_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let cli = Cli {
            media_root: Some(media),
            ..Cli::parse_from(["media-catalog"])
        };
        let error = Config::resolve(cli, FileConfig::default(), 4).unwrap_err();
        assert!(error.to_string().contains("--data-root"));
    }

    #[test]
    fn missing_state_root_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        let data = temp.path().join("data");
        std::fs::create_dir_all(&media).unwrap();
        let cli = Cli {
            media_root: Some(media),
            data_root: Some(data),
            ..Cli::parse_from(["media-catalog"])
        };
        let error = Config::resolve(cli, FileConfig::default(), 4).unwrap_err();
        assert!(error.to_string().contains("--state-root"));
    }

    #[test]
    fn prefixes_are_normalized() {
        assert_eq!(
            normalize_prefix("media/".into(), "--media-url-prefix").unwrap(),
            "/media"
        );
        assert_eq!(
            normalize_prefix("family-media".into(), "--media-url-prefix").unwrap(),
            "/family-media"
        );
        assert!(normalize_prefix("/".into(), "--media-url-prefix").is_err());
        assert!(normalize_prefix("".into(), "--media-url-prefix").is_err());
        assert!(normalize_prefix("//media".into(), "--media-url-prefix").is_err());
        assert!(normalize_prefix("/a/../b".into(), "--media-url-prefix").is_err());
        assert!(normalize_prefix("/a/./b".into(), "--media-url-prefix").is_err());
    }

    #[test]
    fn loads_toml_file_and_rejects_unknown_keys() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(
            &path,
            "media_root = \"./media\"\ndata_root = \"./data\"\nstate_root = \"./state\"\njobs = 3\n",
        )
        .unwrap();
        let file = FileConfig::load(Some(&path)).unwrap();
        assert_eq!(file.media_root, Some(PathBuf::from("./media")));
        assert_eq!(file.data_root, Some(PathBuf::from("./data")));
        assert_eq!(file.state_root, Some(PathBuf::from("./state")));
        assert_eq!(file.jobs, Some(3));

        std::fs::write(&path, "media_root = \"./media\"\nnot_a_flag = true\n").unwrap();
        let error = FileConfig::load(Some(&path)).unwrap_err();
        assert!(format!("{error:#}").contains("unknown field"));
    }

    #[test]
    fn parses_server_keys_from_the_shared_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(
            &path,
            "media_root = \"./media\"\nbind = \"0.0.0.0:9090\"\nmax_streams = 3\n",
        )
        .unwrap();
        let file = FileConfig::load(Some(&path)).unwrap();
        assert_eq!(file.bind.as_deref(), Some("0.0.0.0:9090"));
        assert_eq!(file.max_streams, Some(3));
    }

    #[test]
    fn generator_resolves_when_the_file_contains_server_keys() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(
            &path,
            format!(
                "media_root = \"{}\"\ndata_root = \"{}\"\nstate_root = \"{}\"\nbind = \"127.0.0.1:9000\"\nmax_streams = 2\n",
                media.display(),
                temp.path().join("data").display(),
                temp.path().join("state").display()
            ),
        )
        .unwrap();

        let file = FileConfig::load(Some(&path)).unwrap();
        let config = Config::resolve(Cli::parse_from(["media-catalog"]), file, 2).expect("resolve");
        assert_eq!(config.media_root, media);
        assert_eq!(config.data_root, temp.path().join("data"));
        assert_eq!(config.state_root, temp.path().join("state"));
    }

    #[test]
    fn legacy_catalog_root_key_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(
            &path,
            "media_root = \"./media\"\ncatalog_root = \"./catalog\"\n",
        )
        .unwrap();
        let error = FileConfig::load(Some(&path)).unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("unknown field"), "error was: {message}");
        assert!(message.contains("catalog_root"), "error was: {message}");
    }

    #[test]
    fn equal_roots_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        let state = temp.path().join("state");
        std::fs::create_dir_all(&media).unwrap();
        let error = Config::resolve(
            cli_with_roots(
                media.to_str().unwrap(),
                media.to_str().unwrap(),
                state.to_str().unwrap(),
            ),
            FileConfig::default(),
            2,
        )
        .unwrap_err();
        assert!(error.to_string().contains("separate directories"));
    }

    #[test]
    fn state_inside_data_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        let data = temp.path().join("data");
        let state = data.join("state");
        std::fs::create_dir_all(&media).unwrap();
        let error = Config::resolve(
            cli_with_roots(
                media.to_str().unwrap(),
                data.to_str().unwrap(),
                state.to_str().unwrap(),
            ),
            FileConfig::default(),
            2,
        )
        .unwrap_err();
        assert!(error.to_string().contains("separate directories"));
    }

    #[test]
    fn data_inside_media_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        let data = media.join("data");
        let state = temp.path().join("state");
        std::fs::create_dir_all(&media).unwrap();
        let error = Config::resolve(
            cli_with_roots(
                media.to_str().unwrap(),
                data.to_str().unwrap(),
                state.to_str().unwrap(),
            ),
            FileConfig::default(),
            2,
        )
        .unwrap_err();
        assert!(error.to_string().contains("separate directories"));
    }

    #[test]
    fn state_inside_media_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        let data = temp.path().join("data");
        let state = media.join("state");
        std::fs::create_dir_all(&media).unwrap();
        let error = Config::resolve(
            cli_with_roots(
                media.to_str().unwrap(),
                data.to_str().unwrap(),
                state.to_str().unwrap(),
            ),
            FileConfig::default(),
            2,
        )
        .unwrap_err();
        assert!(error.to_string().contains("separate directories"));
    }

    #[test]
    fn valid_sibling_layout_succeeds() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        let data = temp.path().join("data");
        let state = temp.path().join("state");
        std::fs::create_dir_all(&media).unwrap();
        let config = Config::resolve(
            cli_with_roots(
                media.to_str().unwrap(),
                data.to_str().unwrap(),
                state.to_str().unwrap(),
            ),
            FileConfig::default(),
            2,
        )
        .unwrap();
        assert_eq!(config.media_root, media);
        assert_eq!(config.data_root, data);
        assert_eq!(config.state_root, state);
        assert_eq!(config.state_db, state.join("state.sqlite"));
    }

    #[test]
    fn data_root_must_be_a_directory_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        let data = temp.path().join("data");
        let state = temp.path().join("state");
        std::fs::create_dir_all(&media).unwrap();
        std::fs::write(&data, b"not a directory").unwrap();
        let error = Config::resolve(
            cli_with_roots(
                media.to_str().unwrap(),
                data.to_str().unwrap(),
                state.to_str().unwrap(),
            ),
            FileConfig::default(),
            2,
        )
        .unwrap_err();
        assert!(error.to_string().contains("data root is not a directory"));
    }

    #[test]
    fn roots_own_data_and_state_paths() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        let data = temp.path().join("data");
        let state = temp.path().join("state");
        std::fs::create_dir_all(&media).unwrap();
        let config = Config::with_roots(&media, &data, &state);
        assert_eq!(config.data_root, data);
        assert_eq!(config.state_root, state);
        assert_eq!(config.state_db, state.join("state.sqlite"));
        assert!(config.events_file.is_none());
    }

    #[test]
    fn events_cli_flag_overrides_toml_value() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let toml_events = temp.path().join("toml-events.toml");
        let cli_events = temp.path().join("cli-events.toml");
        std::fs::write(&toml_events, "").unwrap();
        std::fs::write(&cli_events, "").unwrap();

        let file = FileConfig {
            events_file: Some(toml_events),
            ..FileConfig::default()
        };
        let config = Config::resolve(
            Cli {
                events: Some(cli_events.clone()),
                ..cli_with_roots(
                    media.to_str().unwrap(),
                    temp.path().join("data").to_str().unwrap(),
                    temp.path().join("state").to_str().unwrap(),
                )
            },
            file,
            2,
        )
        .unwrap();
        assert_eq!(config.events_file.as_deref(), Some(cli_events.as_path()));
    }

    #[test]
    fn events_file_from_toml_is_used() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let events = temp.path().join("events.toml");
        std::fs::write(&events, "").unwrap();

        let path = temp.path().join("media-catalog.toml");
        std::fs::write(
            &path,
            format!(
                "media_root = \"{}\"\ndata_root = \"{}\"\nstate_root = \"{}\"\nevents_file = \"{}\"\n",
                media.display(),
                temp.path().join("data").display(),
                temp.path().join("state").display(),
                events.display()
            ),
        )
        .unwrap();
        let file = FileConfig::load(Some(&path)).unwrap();
        let config = Config::resolve(Cli::parse_from(["media-catalog"]), file, 2).unwrap();
        assert_eq!(config.events_file.as_deref(), Some(events.as_path()));
    }

    #[test]
    fn neither_events_source_yields_no_events() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let config = Config::resolve(
            cli_with_roots(
                media.to_str().unwrap(),
                temp.path().join("data").to_str().unwrap(),
                temp.path().join("state").to_str().unwrap(),
            ),
            FileConfig::default(),
            2,
        )
        .unwrap();
        assert!(config.events_file.is_none());
    }

    #[test]
    fn absent_events_path_yields_no_events() {
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(resolve_events_path(None, temp.path()).unwrap(), None);
    }

    #[test]
    fn missing_configured_events_file_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let file = FileConfig {
            events_file: Some(temp.path().join("events.toml")),
            ..FileConfig::default()
        };
        let error = Config::resolve(
            cli_with_roots(
                media.to_str().unwrap(),
                temp.path().join("data").to_str().unwrap(),
                temp.path().join("state").to_str().unwrap(),
            ),
            file,
            2,
        )
        .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("events file not found"), "{message}");
    }

    #[test]
    fn relative_configured_events_path_is_resolved_against_cwd() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let path = nested.join("events.toml");
        std::fs::write(&path, "").unwrap();
        let resolved =
            resolve_events_path(Some(Path::new("nested/events.toml")), temp.path()).unwrap();
        assert_eq!(resolved.as_deref(), Some(path.as_path()));
    }

    #[test]
    fn directory_events_path_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("events");
        std::fs::create_dir_all(&directory).unwrap();
        let error = resolve_events_path(Some(&directory), temp.path()).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("events path is not a file"), "{message}");
    }

    #[test]
    fn explicit_events_file_is_used() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("custom.toml");
        std::fs::write(&path, "").unwrap();
        let resolved = resolve_events_path(Some(Path::new("custom.toml")), temp.path()).unwrap();
        assert_eq!(resolved.as_deref(), Some(path.as_path()));
    }

    #[test]
    fn missing_explicit_events_file_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let error = resolve_events_path(Some(Path::new("missing.toml")), temp.path()).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("events file not found"), "{message}");
        assert!(message.contains("missing.toml"), "{message}");
    }

    #[test]
    fn events_only_flag_enables_events_only_mode() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let cli = Cli {
            events_only: true,
            ..cli_with_roots(
                media.to_str().unwrap(),
                temp.path().join("data").to_str().unwrap(),
                temp.path().join("state").to_str().unwrap(),
            )
        };
        let config = Config::resolve(cli, FileConfig::default(), 2).unwrap();
        assert!(config.events_only);
    }

    #[test]
    fn events_only_toml_key_enables_events_only_mode() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let file = FileConfig {
            events_only: Some(true),
            ..FileConfig::default()
        };
        let config = Config::resolve(
            cli_with_roots(
                media.to_str().unwrap(),
                temp.path().join("data").to_str().unwrap(),
                temp.path().join("state").to_str().unwrap(),
            ),
            file,
            2,
        )
        .unwrap();
        assert!(config.events_only);
    }

    #[test]
    fn events_only_flag_wins_over_toml_false() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let file = FileConfig {
            events_only: Some(false),
            ..FileConfig::default()
        };
        let cli = Cli {
            events_only: true,
            ..cli_with_roots(
                media.to_str().unwrap(),
                temp.path().join("data").to_str().unwrap(),
                temp.path().join("state").to_str().unwrap(),
            )
        };
        let config = Config::resolve(cli, file, 2).unwrap();
        assert!(config.events_only);
    }

    #[test]
    fn events_only_defaults_to_off() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let config = Config::resolve(
            cli_with_roots(
                media.to_str().unwrap(),
                temp.path().join("data").to_str().unwrap(),
                temp.path().join("state").to_str().unwrap(),
            ),
            FileConfig::default(),
            2,
        )
        .unwrap();
        assert!(!config.events_only);
    }

    #[test]
    fn server_parses_events_only_key_from_shared_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(
            &path,
            "media_root = \"./media\"\nevents_only = true\nbind = \"0.0.0.0:9090\"\n",
        )
        .unwrap();
        let file = FileConfig::load(Some(&path)).unwrap();
        assert_eq!(file.events_only, Some(true));
        assert_eq!(file.bind.as_deref(), Some("0.0.0.0:9090"));
    }

    #[test]
    fn events_only_skips_media_root_existence_check() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("nonexistent-media");
        let mut config =
            Config::with_roots(&media, temp.path().join("data"), temp.path().join("state"));
        config.events_only = true;
        config
            .validate()
            .expect("events-only validates without media root");
    }

    #[test]
    fn full_build_still_requires_media_root_to_exist() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("nonexistent-media");
        let config =
            Config::with_roots(&media, temp.path().join("data"), temp.path().join("state"));
        let error = config.validate().unwrap_err();
        assert!(
            error.to_string().contains("media root does not exist"),
            "{error}"
        );
    }

    #[test]
    fn events_only_still_rejects_non_directory_media_root_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media-file");
        std::fs::write(&media, b"not a dir").unwrap();
        let mut config =
            Config::with_roots(&media, temp.path().join("data"), temp.path().join("state"));
        config.events_only = true;
        config
            .validate()
            .expect("events-only skips is_dir check too");
    }

    #[test]
    fn include_location_flag_enables_location_publication() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let cli = Cli {
            include_location: true,
            ..cli_with_roots(
                media.to_str().unwrap(),
                temp.path().join("data").to_str().unwrap(),
                temp.path().join("state").to_str().unwrap(),
            )
        };
        let config = Config::resolve(cli, FileConfig::default(), 2).unwrap();
        assert!(config.include_location);
    }

    #[test]
    fn include_location_toml_key_enables_location_publication() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let file = FileConfig {
            include_location: Some(true),
            ..FileConfig::default()
        };
        let config = Config::resolve(
            cli_with_roots(
                media.to_str().unwrap(),
                temp.path().join("data").to_str().unwrap(),
                temp.path().join("state").to_str().unwrap(),
            ),
            file,
            2,
        )
        .unwrap();
        assert!(config.include_location);
    }

    #[test]
    fn include_location_flag_wins_over_toml_false() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let file = FileConfig {
            include_location: Some(false),
            ..FileConfig::default()
        };
        let cli = Cli {
            include_location: true,
            ..cli_with_roots(
                media.to_str().unwrap(),
                temp.path().join("data").to_str().unwrap(),
                temp.path().join("state").to_str().unwrap(),
            )
        };
        let config = Config::resolve(cli, file, 2).unwrap();
        assert!(config.include_location);
    }

    #[test]
    fn include_location_defaults_to_off() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let config = Config::resolve(
            cli_with_roots(
                media.to_str().unwrap(),
                temp.path().join("data").to_str().unwrap(),
                temp.path().join("state").to_str().unwrap(),
            ),
            FileConfig::default(),
            2,
        )
        .unwrap();
        assert!(!config.include_location);
    }

    #[test]
    fn server_parses_include_location_and_ignores_it() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(
            &path,
            "media_root = \"./media\"\ninclude_location = true\nbind = \"0.0.0.0:9090\"\n",
        )
        .unwrap();
        let file = FileConfig::load(Some(&path)).unwrap();
        assert_eq!(file.include_location, Some(true));
        assert_eq!(file.bind.as_deref(), Some("0.0.0.0:9090"));
    }

    #[test]
    fn include_location_rejects_non_boolean_values() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(&path, "include_location = \"yes\"\n").unwrap();
        let error = FileConfig::load(Some(&path)).unwrap_err();
        assert!(
            format!("{error:#}").contains("include_location"),
            "{error:#}"
        );
    }
}
