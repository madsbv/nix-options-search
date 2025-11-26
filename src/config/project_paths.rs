/// Global definitions of project file locations.
/// Each definition is wrapped in `OnceLock` to prevent locations changing under us while the program is running.
use directories::ProjectDirs;
use std::path::PathBuf;

/// `"NOX"`
pub(super) fn project_env_name() -> String {
    PROJECT_NAME.to_uppercase()
}

/// `"nox"`
static PROJECT_NAME: &str = env!("CARGO_BIN_NAME");

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("dev", "mvil", PROJECT_NAME)
}

/// Cache location is determined by the first found option of:
/// - the environment variable `NOX_CACHE`,
/// - The OS standard cache directory (usually in `$HOME/.cache/nox` on Linux, `$HOME/Library/Caches/` on Mac),
///
/// If neither is found, the cache will be placed in the subdirectory `.cache` in the current directory, which will be created if it does not exist.
pub(super) fn default_cache_dir() -> PathBuf {
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
}

/// Log location is determined by the first found option of:
/// - the environment variable `NOX_LOG`,
/// - The OS standard data directory (usually in `$HOME/.local/share/` on Linux, `$HOME/Library/Application Support/` on Mac),
///
/// If neither is found, the log will be placed in the subdirectory `.log` in the current directory, which will be created if it does not exist.
pub(super) fn default_log_file() -> PathBuf {
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
    .join(format!("{PROJECT_NAME}.log"))
}

/// Config file location is determined by the first found option of:
/// - the environment variable `NOX_CONFIG`,
/// - The OS standard data directory (usually in `$HOME/.config/nox/` on Linux, `$HOME/Library/Application Support/` on Mac),
///
/// If neither is found, try to read a nox.toml in the current directory, which will be created if it does not exist.
pub(crate) fn default_config_file() -> PathBuf {
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
    .join(format!("{PROJECT_NAME}.toml"))
}
