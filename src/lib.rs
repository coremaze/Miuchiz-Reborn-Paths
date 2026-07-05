//! Shared storage location policy for Miuchiz Reborn applications.
//!
//! Each app passes its own name and gets a per-app directory under a single
//! "Miuchiz Reborn" umbrella, with each category in its OS-correct root:
//!
//! | Category | macOS | Windows | Linux |
//! |----------|-------|---------|-------|
//! | config | `~/Library/Application Support/Miuchiz Reborn/<app>/` | `%APPDATA%\Miuchiz Reborn\<app>\`      | `~/.config/miuchiz-reborn/<app>/` |
//! | data   | `~/Library/Application Support/Miuchiz Reborn/<app>/` | `%APPDATA%\Miuchiz Reborn\<app>\`      | `~/.local/share/miuchiz-reborn/<app>/` |
//! | cache  | `~/Library/Caches/Miuchiz Reborn/<app>/`             | `%LOCALAPPDATA%\Miuchiz Reborn\<app>\` | `~/.cache/miuchiz-reborn/<app>/` |
//! | state  | `~/Library/Application Support/Miuchiz Reborn/<app>/` | `%LOCALAPPDATA%\Miuchiz Reborn\<app>\` | `~/.local/state/miuchiz-reborn/<app>/` |
//!
//! macOS and Windows map several categories to the same root; apps namespace
//! their own files within. `MIUCHIZ_REBORN_HOME` reroots every category under
//! one directory (`<home>/{config,data,cache,state}/<app>`) for portable
//! installs and tests.

use std::path::{Path, PathBuf};

use directories::BaseDirs;

const UMBRELLA_HUMAN: &str = "Miuchiz Reborn";
const UMBRELLA_XDG: &str = "miuchiz-reborn";

/// Reroots every category under one directory (portable installs, tests).
pub const ENV_HOME: &str = "MIUCHIZ_REBORN_HOME";

fn umbrella() -> &'static str {
    if cfg!(any(target_os = "macos", target_os = "windows")) {
        UMBRELLA_HUMAN
    } else {
        UMBRELLA_XDG
    }
}

/// Resolved storage directories for one Miuchiz Reborn application.
#[derive(Debug, Clone)]
pub struct AppDirs {
    app: String,
    config: PathBuf,
    data: PathBuf,
    cache: PathBuf,
    state: PathBuf,
}

impl AppDirs {
    /// Resolve directories for `app` (a plain slug like `"launcher"`).
    ///
    /// Falls back to a local `./.miuchiz-reborn` tree when there is no home
    /// directory and no [`ENV_HOME`] override.
    pub fn new(app: impl Into<String>) -> Self {
        let app = app.into();
        debug_assert!(!app.is_empty(), "app name must not be empty");
        debug_assert!(
            !app.contains(['/', '\\']),
            "app name must be a plain slug, not a path"
        );
        let [config, data, cache, state] = resolve(&app);
        Self {
            app,
            config,
            data,
            cache,
            state,
        }
    }

    /// The app name this was constructed with.
    pub fn app(&self) -> &str {
        &self.app
    }

    /// User settings. Roams on Windows.
    pub fn config_dir(&self) -> &Path {
        &self.config
    }

    /// Durable user data. Roams on Windows.
    pub fn data_dir(&self) -> &Path {
        &self.data
    }

    /// Regenerable cache.
    pub fn cache_dir(&self) -> &Path {
        &self.cache
    }

    /// Logs and machine-local bookkeeping.
    pub fn state_dir(&self) -> &Path {
        &self.state
    }
}

/// Create a directory and its parents; idempotent.
pub fn ensure(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

fn resolve(app: &str) -> [PathBuf; 4] {
    if let Some(home) = std::env::var_os(ENV_HOME).filter(|v| !v.is_empty()) {
        return single_root_layout(Path::new(&home), app);
    }
    match BaseDirs::new() {
        Some(base) => os_layout(
            base.config_dir(),
            base.data_dir(),
            base.cache_dir(),
            // macOS/Windows have no dedicated state dir; use local data there.
            base.state_dir().unwrap_or_else(|| base.data_local_dir()),
            app,
        ),
        None => single_root_layout(Path::new(".miuchiz-reborn"), app),
    }
}

/// `[config, data, cache, state]` as `<root>/<umbrella>/<app>` per OS root.
fn os_layout(config: &Path, data: &Path, cache: &Path, state: &Path, app: &str) -> [PathBuf; 4] {
    let umb = umbrella();
    let leaf = |root: &Path| root.join(umb).join(app);
    [leaf(config), leaf(data), leaf(cache), leaf(state)]
}

/// `[config, data, cache, state]` as `<home>/<category>/<app>` under one root.
fn single_root_layout(home: &Path, app: &str) -> [PathBuf; 4] {
    let leaf = |cat: &str| home.join(cat).join(app);
    [leaf("config"), leaf("data"), leaf("cache"), leaf("state")]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_root_layout_splits_category_then_app() {
        let dirs = single_root_layout(Path::new("/portable/mr"), "launcher");
        assert_eq!(dirs[0], Path::new("/portable/mr/config/launcher"));
        assert_eq!(dirs[1], Path::new("/portable/mr/data/launcher"));
        assert_eq!(dirs[2], Path::new("/portable/mr/cache/launcher"));
        assert_eq!(dirs[3], Path::new("/portable/mr/state/launcher"));
    }

    #[test]
    fn os_layout_inserts_umbrella_and_app() {
        let dirs = os_layout(
            Path::new("/cfg"),
            Path::new("/dat"),
            Path::new("/cch"),
            Path::new("/stt"),
            "browser",
        );
        let umb = umbrella();
        assert_eq!(dirs[0], Path::new("/cfg").join(umb).join("browser"));
        assert_eq!(dirs[3], Path::new("/stt").join(umb).join("browser"));
        for d in &dirs {
            assert!(d.ends_with(Path::new(umb).join("browser")));
        }
    }

    #[test]
    fn umbrella_is_human_or_xdg_per_os() {
        let expected = if cfg!(any(target_os = "macos", target_os = "windows")) {
            "Miuchiz Reborn"
        } else {
            "miuchiz-reborn"
        };
        assert_eq!(umbrella(), expected);
    }
}
