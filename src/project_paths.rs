/// Global definitions of project file locations.
/// Each definition is wrapped in `OnceLock` to prevent locations changing under us while the program is running.
use directories::ProjectDirs;
use std::path::PathBuf;
use std::sync::OnceLock;

/// `"NOX"`
static PROJECT_ENV_NAME: OnceLock<String> = OnceLock::new();
pub fn project_env_name() -> &'static str {
    PROJECT_ENV_NAME.get_or_init(|| project_name().to_uppercase())
}

/// `"nox"`
static PROJECT_NAME: OnceLock<String> = OnceLock::new();
pub fn project_name() -> &'static str {
    PROJECT_NAME.get_or_init(|| env!("CARGO_BIN_NAME").to_string())
}

static PROJECT_DIRECTORY: OnceLock<Option<ProjectDirs>> = OnceLock::new();
fn project_directory() -> Option<&'static ProjectDirs> {
    PROJECT_DIRECTORY
        .get_or_init(|| ProjectDirs::from("dev", "mvil", project_name()))
        .as_ref()
}

/// Cache location is determined by the first found option of:
/// - the environment variable `NOX_CACHE`,
/// - The OS standard cache directory (usually in `$HOME/.cache/nox` on Linux, `$HOME/Library/Caches/` on Mac),
///
/// If neither is found, the cache will be placed in the subdirectory `.cache` in the current directory, which will be created if it does not exist.
static CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();
pub fn cache_dir() -> &'static PathBuf {
    CACHE_DIR.get_or_init(|| {
        let dir = std::env::var(format!("{}_CACHE", project_env_name()))
            .ok()
            .map(PathBuf::from);
        if let Some(s) = dir {
            s
        } else if let Some(proj_dirs) = project_directory() {
            proj_dirs.cache_dir().to_path_buf()
        } else {
            PathBuf::from(".").join(".cache")
        }
    })
}

/// Log location is determined by the first found option of:
/// - the environment variable `NOX_LOG`,
/// - The OS standard data directory (usually in `$HOME/.local/share/` on Linux, `$HOME/Library/Application Support/` on Mac),
///
/// If neither is found, the log will be placed in the subdirectory `.log` in the current directory, which will be created if it does not exist.
static LOG_FILE: OnceLock<PathBuf> = OnceLock::new();
pub fn log_file() -> &'static PathBuf {
    LOG_FILE.get_or_init(|| {
        let dir = std::env::var(format!("{}_LOG", project_env_name()))
            .ok()
            .map(PathBuf::from);
        if let Some(s) = dir {
            s
        } else if let Some(proj_dirs) = project_directory() {
            proj_dirs.data_dir().to_path_buf()
        } else {
            PathBuf::from(".").join(".log")
        }
        .join(format!("{}.log", project_name()))
    })
}

/// Config file location is determined by the first found option of:
/// - the environment variable `NOX_CONFIG`,
/// - The OS standard data directory (usually in `$HOME/.config/nox/` on Linux, `$HOME/Library/Application Support/` on Mac),
///
/// If neither is found, try to read a nox.toml in the current directory, which will be created if it does not exist.
static CONFIG_FILE: OnceLock<PathBuf> = OnceLock::new();
pub fn config_file() -> &'static PathBuf {
    CONFIG_FILE.get_or_init(|| {
        let dir = std::env::var(format!("{}_CONFIG", project_env_name()))
            .ok()
            .map(PathBuf::from);
        if let Some(s) = dir {
            s
        } else if let Some(proj_dirs) = project_directory() {
            proj_dirs.config_dir().to_path_buf()
        } else {
            PathBuf::from(".")
        }
        .join(format!("{}.toml", project_name()))
    })
}
