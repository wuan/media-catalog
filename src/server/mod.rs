pub mod assets;
pub mod catalog;
pub mod media;
pub mod static_files;
mod support;

use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use axum::extract::{Request, State};
use axum::http::{header, HeaderName, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use clap::Parser;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::set_header::SetResponseHeaderLayer;

use crate::config::FileConfig;
use catalog::CatalogStore;
use support::{error_response, text_response};

/// Loopback address used when neither flags nor the configuration file set one.
const DEFAULT_BIND: &str = "127.0.0.1:8080";
const DEFAULT_MAX_STREAMS: usize = 8;

/// Runtime configuration for the read-only server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub data_root: PathBuf,
    pub media_root: PathBuf,
    pub bind: SocketAddr,
    pub token: Option<String>,
    pub max_streams: usize,
}

impl ServerConfig {
    /// Build a configuration with the default loopback bind and no access token.
    pub fn new(data_root: impl Into<PathBuf>, media_root: impl Into<PathBuf>) -> Self {
        Self {
            data_root: data_root.into(),
            media_root: media_root.into(),
            bind: DEFAULT_BIND
                .parse()
                .expect("the default bind address is valid"),
            token: None,
            max_streams: DEFAULT_MAX_STREAMS,
        }
    }

    /// Resolve server settings from CLI flags first, then the shared TOML
    /// file, then built-in defaults. Paths stay as configured; `validate`
    /// canonicalizes them against the working directory.
    pub fn resolve(cli: ServeCli, file: FileConfig) -> Result<Self> {
        let data_root = cli.data_root.or(file.data_root).ok_or_else(|| {
            anyhow::anyhow!("--data-root is required (or set data_root in media-catalog.toml)")
        })?;
        let media_root = cli.media_root.or(file.media_root).ok_or_else(|| {
            anyhow::anyhow!("--media-root is required (or set media_root in media-catalog.toml)")
        })?;

        let bind = match cli.bind.or(file.bind) {
            Some(value) => value.parse::<SocketAddr>().with_context(|| {
                format!(
                    "invalid bind address {value:?}: --bind or bind in media-catalog.toml must be a socket address such as 127.0.0.1:8080"
                )
            })?,
            None => DEFAULT_BIND.parse().expect("the default bind address is valid"),
        };
        let max_streams = cli
            .max_streams
            .or(file.max_streams)
            .unwrap_or(DEFAULT_MAX_STREAMS);

        Ok(Self {
            data_root,
            media_root,
            bind,
            token: None,
            max_streams,
        })
    }

    /// Canonicalize and validate the roots before serving. The data root may
    /// be absent until the generator publishes a catalog; the media root is
    /// required because `/media/` depends on it.
    pub fn validate(self) -> Result<Self> {
        if self.max_streams == 0 {
            bail!("--max-streams must be at least 1 (or set max_streams in media-catalog.toml)");
        }

        let data_root = match std::fs::canonicalize(&self.data_root) {
            Ok(path) => {
                if !path.is_dir() {
                    bail!("data root is not a directory: {}", path.display());
                }
                path
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let absolute = if self.data_root.is_absolute() {
                    self.data_root.clone()
                } else {
                    std::env::current_dir()
                        .context("reading the current directory")?
                        .join(&self.data_root)
                };
                crate::config::normalize_lexically(&absolute)
            }
            Err(error) => {
                return Err(anyhow::Error::new(error)
                    .context(format!("resolving data root {}", self.data_root.display())));
            }
        };

        let media_root = std::fs::canonicalize(&self.media_root)
            .with_context(|| format!("media root does not exist: {}", self.media_root.display()))?;
        if !media_root.is_dir() {
            bail!("media root is not a directory: {}", media_root.display());
        }

        if data_root.starts_with(&media_root) || media_root.starts_with(&data_root) {
            bail!(
                "data root and media root must be separate directories: {} and {}",
                data_root.display(),
                media_root.display()
            );
        }

        Ok(Self {
            data_root,
            media_root,
            ..self
        })
    }
}

/// Command-line interface for the runtime server.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "media-catalog-server",
    version,
    about = "Render the published media catalog and serve read-only originals over HTTP"
)]
pub struct ServeCli {
    /// Root for published catalog data. Contains `catalog/` and `thumbs/`.
    /// Overrides `data_root` in the shared configuration file.
    #[arg(long, value_name = "PATH")]
    pub data_root: Option<PathBuf>,

    /// Root of the read-only media archive.
    /// Overrides `media_root` in the shared configuration file.
    #[arg(long, value_name = "PATH")]
    pub media_root: Option<PathBuf>,

    /// Address to listen on. Overrides `bind` in the shared configuration file.
    /// Defaults to loopback only.
    #[arg(long, value_name = "ADDR")]
    pub bind: Option<String>,

    /// Maximum concurrent `/media/` streams.
    /// Overrides `max_streams` in the shared configuration file. Defaults to 8.
    #[arg(long, value_name = "N")]
    pub max_streams: Option<usize>,

    /// Optional TOML configuration file. Defaults to ./media-catalog.toml when present.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

impl ServeCli {
    /// Load the shared configuration file, resolve the server settings, and
    /// read the access token from `MEDIA_CATALOG_TOKEN` rather than the command
    /// line.
    pub fn into_config(self) -> Result<ServerConfig> {
        let file = FileConfig::load(self.config.as_deref())?;
        let mut config = ServerConfig::resolve(self, file)?;
        config.token = token_from_env();
        Ok(config)
    }
}

/// Access control is enabled only by a non-empty `MEDIA_CATALOG_TOKEN`.
fn token_from_env() -> Option<String> {
    normalize_token(std::env::var("MEDIA_CATALOG_TOKEN").ok())
}

fn normalize_token(value: Option<String>) -> Option<String> {
    value.filter(|token| !token.is_empty())
}

#[derive(Clone)]
pub struct AppState {
    pub(crate) data_root: PathBuf,
    pub(crate) media_root: PathBuf,
    pub(crate) token: Option<Arc<str>>,
    pub(crate) media_streams: Arc<Semaphore>,
    pub(crate) catalog: Arc<CatalogStore>,
}

impl AppState {
    fn from_config(config: ServerConfig) -> Self {
        Self {
            catalog: Arc::new(CatalogStore::new(config.data_root.clone())),
            data_root: config.data_root,
            media_root: config.media_root,
            token: config.token.map(Arc::from),
            media_streams: Arc::new(Semaphore::new(config.max_streams)),
        }
    }

    /// Number of available media-stream slots. Useful for tests and diagnostics.
    pub fn available_streams(&self) -> usize {
        self.media_streams.available_permits()
    }

    /// Try to reserve a media-stream slot. The handler uses the same mechanism.
    pub fn try_acquire_stream(&self) -> Option<tokio::sync::OwnedSemaphorePermit> {
        self.media_streams.clone().try_acquire_owned().ok()
    }
}

/// Validate configuration and build its shared state.
pub fn app_state(config: ServerConfig) -> Result<AppState> {
    Ok(AppState::from_config(config.validate()?))
}

/// Build the HTTP router, validating the configured roots.
pub fn router(config: ServerConfig) -> Result<Router> {
    Ok(router_from_state(app_state(config)?))
}

/// Build the HTTP router from existing state. Useful for embedding and tests.
pub fn router_from_state(state: AppState) -> Router {
    let nosniff = HeaderName::from_static("x-content-type-options");
    let router = Router::new()
        .route("/healthz", get(health))
        .route("/", get(catalog::serve_root))
        .route("/search.html", get(assets::serve_search_shell))
        .route("/assets/site.css", get(assets::serve_site_css))
        .route("/assets/search.js", get(assets::serve_search_js))
        .route("/assets/viewer.js", get(assets::serve_viewer_js))
        // Explicit event routes are registered before the numeric year/month
        // routes so `/events/` never falls through to `/{year}/`.
        .route("/events", get(catalog::serve_events))
        .route("/events/", get(catalog::serve_events))
        .route("/events/{slug}", get(catalog::serve_event))
        .route("/events/{slug}/", get(catalog::serve_event))
        // Dynamic catalog routes accept numeric segments in the handlers and are
        // registered before the static fallback.
        .route("/{year}", get(catalog::serve_year))
        .route("/{year}/", get(catalog::serve_year))
        .route("/{year}/{month}", get(catalog::serve_month))
        .route("/{year}/{month}/", get(catalog::serve_month))
        .route("/catalog/{*path}", get(static_files::serve_static))
        .route("/thumbs/{*path}", get(static_files::serve_static))
        .route("/media/{*path}", get(media::serve_media))
        // Everything else is not part of the runtime contract: obsolete static
        // HTML and asset files in the data root are not served.
        .fallback(not_found)
        .with_state(state.clone());

    router
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .layer(middleware::from_fn(log_requests))
        .layer(SetResponseHeaderLayer::overriding(
            nosniff,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(CatchPanicLayer::new())
}

/// Bind and serve until a shutdown signal arrives.
pub async fn serve(config: ServerConfig) -> Result<()> {
    if !config.media_root.is_dir() {
        eprintln!(
            "warning: media root is unavailable: {}; startup validation will fail",
            config.media_root.display()
        );
    }
    let config = config.validate()?;
    let listener = TcpListener::bind(config.bind)
        .await
        .with_context(|| format!("binding {}", config.bind))?;
    let local = listener.local_addr().context("reading listener address")?;

    eprintln!("media-catalog-server listening on http://{local}");
    eprintln!("  data:    {}", config.data_root.display());
    eprintln!("  media:   {}", config.media_root.display());
    if !config.media_root.is_dir() {
        eprintln!("warning: media root is unavailable; /media requests will return 503");
    }

    serve_listener(listener, config, shutdown_signal()).await
}

/// Serve on an already-bound listener with a caller-provided shutdown future.
pub async fn serve_listener(
    listener: TcpListener,
    config: ServerConfig,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<()> {
    let app = router(config)?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .context("serving HTTP requests")?;
    Ok(())
}

async fn not_found() -> Response {
    error_response(StatusCode::NOT_FOUND, "not found")
}

async fn health(State(state): State<AppState>) -> Response {
    let catalog_ok = state.catalog.current().await.is_some();
    let media_ok = state.media_root.is_dir();
    if catalog_ok && media_ok {
        text_response(StatusCode::OK, "ok\n", "no-store")
    } else if !catalog_ok && !media_ok {
        text_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "catalog and media unavailable\n",
            "no-store",
        )
    } else if !catalog_ok {
        text_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "catalog unavailable\n",
            "no-store",
        )
    } else {
        text_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "media unavailable\n",
            "no-store",
        )
    }
}

async fn require_auth(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let Some(expected) = state.token.as_deref() else {
        return next.run(request).await;
    };

    let provided = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    if provided
        .map(|provided| constant_time_eq(provided, expected))
        .unwrap_or(false)
    {
        next.run(request).await
    } else {
        let mut response = error_response(StatusCode::UNAUTHORIZED, "unauthorized");
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        response
    }
}

async fn log_requests(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let started = Instant::now();
    let response = next.run(request).await;
    let status = response.status();
    let bytes = response
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    eprintln!(
        "{} {} -> {} ({} bytes, {} ms)",
        method,
        path,
        status.as_u16(),
        bytes,
        started.elapsed().as_millis()
    );
    response
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in left.iter().zip(right) {
        difference |= left ^ right;
    }
    difference == 0
}

async fn shutdown_signal() {
    wait_for_signal().await;
    eprintln!("shutdown requested; draining in-flight requests");

    tokio::spawn(async {
        wait_for_signal().await;
        eprintln!("second shutdown signal received; forcing exit");
        std::process::exit(130);
    });
}

async fn wait_for_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut interrupt = signal(SignalKind::interrupt()).expect("installing SIGINT handler");
        let mut terminate = signal(SignalKind::terminate()).expect("installing SIGTERM handler");
        tokio::select! {
            _ = interrupt.recv() => {}
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_cli() -> ServeCli {
        ServeCli::try_parse_from(["media-catalog-server"]).unwrap()
    }

    #[test]
    fn resolved_defaults_to_loopback_and_eight_streams() {
        let cli = ServeCli::try_parse_from([
            "media-catalog-server",
            "--data-root",
            "/tmp/data",
            "--media-root",
            "/tmp/media",
        ])
        .unwrap();
        let config = ServerConfig::resolve(cli, FileConfig::default()).unwrap();
        assert!(config.bind.ip().is_loopback());
        assert_eq!(config.bind.port(), 8080);
        assert_eq!(config.max_streams, 8);
    }

    #[test]
    fn rejects_missing_data_root() {
        let cli = ServeCli::try_parse_from(["media-catalog-server", "--media-root", "/tmp/media"])
            .unwrap();
        let error = ServerConfig::resolve(cli, FileConfig::default()).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("--data-root"), "error was: {message}");
        assert!(message.contains("data_root"), "error was: {message}");
    }

    #[test]
    fn rejects_missing_media_root() {
        let cli =
            ServeCli::try_parse_from(["media-catalog-server", "--data-root", "/tmp/data"]).unwrap();
        let error = ServerConfig::resolve(cli, FileConfig::default()).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("--media-root"), "error was: {message}");
        assert!(message.contains("media_root"), "error was: {message}");
    }

    #[test]
    fn file_roots_are_used_when_flags_are_absent() {
        let file = FileConfig {
            media_root: Some(PathBuf::from("/srv/media")),
            data_root: Some(PathBuf::from("/srv/data")),
            bind: Some("0.0.0.0:9000".to_string()),
            max_streams: Some(2),
            ..FileConfig::default()
        };
        let config = ServerConfig::resolve(empty_cli(), file).unwrap();
        assert_eq!(config.media_root, PathBuf::from("/srv/media"));
        assert_eq!(config.data_root, PathBuf::from("/srv/data"));
        assert_eq!(config.bind, "0.0.0.0:9000".parse().unwrap());
        assert_eq!(config.max_streams, 2);
    }

    #[test]
    fn cli_values_override_the_file() {
        let file = FileConfig {
            media_root: Some(PathBuf::from("/srv/media")),
            data_root: Some(PathBuf::from("/srv/data")),
            bind: Some("0.0.0.0:9000".to_string()),
            max_streams: Some(2),
            ..FileConfig::default()
        };
        let cli = ServeCli::try_parse_from([
            "media-catalog-server",
            "--media-root",
            "/cli/media",
            "--data-root",
            "/cli/data",
            "--bind",
            "127.0.0.1:7000",
            "--max-streams",
            "4",
        ])
        .unwrap();
        let config = ServerConfig::resolve(cli, file).unwrap();
        assert_eq!(config.media_root, PathBuf::from("/cli/media"));
        assert_eq!(config.data_root, PathBuf::from("/cli/data"));
        assert_eq!(config.bind, "127.0.0.1:7000".parse().unwrap());
        assert_eq!(config.max_streams, 4);
    }

    #[test]
    fn invalid_file_bind_names_flag_and_key() {
        let file = FileConfig {
            media_root: Some(PathBuf::from("/srv/media")),
            data_root: Some(PathBuf::from("/srv/data")),
            bind: Some("not-an-address".to_string()),
            ..FileConfig::default()
        };
        let error = ServerConfig::resolve(empty_cli(), file).unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("--bind"), "error was: {message}");
        assert!(message.contains("bind"), "error was: {message}");
        assert!(message.contains("not-an-address"), "error was: {message}");
    }

    #[test]
    fn invalid_cli_bind_names_flag_and_key() {
        let cli = ServeCli::try_parse_from([
            "media-catalog-server",
            "--media-root",
            "/srv/media",
            "--data-root",
            "/srv/data",
            "--bind",
            "not-an-address",
        ])
        .unwrap();
        let error = ServerConfig::resolve(cli, FileConfig::default()).unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("--bind"), "error was: {message}");
        assert!(
            message.contains("media-catalog.toml"),
            "error was: {message}"
        );
    }

    #[test]
    fn zero_stream_limit_names_flag_and_key_before_binding() {
        let file = FileConfig {
            media_root: Some(PathBuf::from("/srv/media")),
            data_root: Some(PathBuf::from("/srv/data")),
            max_streams: Some(0),
            ..FileConfig::default()
        };
        let config = ServerConfig::resolve(empty_cli(), file).unwrap();
        let error = config.validate().unwrap_err();
        let message = error.to_string();
        assert!(message.contains("--max-streams"), "error was: {message}");
        assert!(message.contains("max_streams"), "error was: {message}");
    }

    #[test]
    fn configured_state_root_is_never_opened() {
        let temp = tempfile::tempdir().unwrap();
        let media = temp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let state = temp.path().join("state-does-not-exist");
        let file = FileConfig {
            media_root: Some(media.clone()),
            data_root: Some(temp.path().join("data")),
            state_root: Some(state.clone()),
            bind: Some("127.0.0.1:9000".to_string()),
            max_streams: Some(2),
            ..FileConfig::default()
        };

        let config = ServerConfig::resolve(empty_cli(), file)
            .unwrap()
            .validate()
            .unwrap();
        assert_eq!(config.media_root, std::fs::canonicalize(&media).unwrap());
        assert_eq!(
            config.data_root,
            crate::config::normalize_lexically(&temp.path().join("data"))
        );
        assert!(!state.exists());
    }

    #[test]
    fn token_is_only_from_a_non_empty_environment_value() {
        assert_eq!(
            normalize_token(Some("secret".to_string())),
            Some("secret".to_string())
        );
        assert_eq!(normalize_token(Some(String::new())), None);
        assert_eq!(normalize_token(None), None);
    }

    #[test]
    fn missing_explicit_config_file_names_the_path() {
        let missing = "/definitely/missing/media-catalog.toml";
        let cli = ServeCli::try_parse_from(["media-catalog-server", "--config", missing]).unwrap();
        let error = cli.into_config().unwrap_err();
        let message = format!("{error:#}");
        assert!(
            message.contains("configuration file not found"),
            "error was: {message}"
        );
        assert!(message.contains(missing), "error was: {message}");
    }

    #[test]
    fn rejects_missing_roots() {
        let config = ServerConfig::new("/definitely/missing/data", "/definitely/missing/media");
        assert!(config.validate().is_err());
    }

    #[test]
    fn compares_tokens_without_an_early_length_match() {
        assert!(constant_time_eq("secret", "secret"));
        assert!(!constant_time_eq("secret", "secreT"));
        assert!(!constant_time_eq("secret", "secret-longer"));
    }
}
