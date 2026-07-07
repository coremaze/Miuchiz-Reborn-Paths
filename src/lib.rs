//! Shared storage location policy for Miuchiz Reborn applications.
//!
//! Each app passes its own name and gets a per-app directory under a single
//! "Miuchiz Reborn" umbrella, with each category in its OS-correct root:
//!
//! | Category | macOS | Windows | Linux |
//! |----------|-------|---------|-------|
//! | config  | `~/Library/Application Support/Miuchiz Reborn/<app>/` | `%APPDATA%\Miuchiz Reborn\<app>\`      | `~/.config/miuchiz-reborn/<app>/` |
//! | data    | `~/Library/Application Support/Miuchiz Reborn/<app>/` | `%APPDATA%\Miuchiz Reborn\<app>\`      | `~/.local/share/miuchiz-reborn/<app>/` |
//! | cache   | `~/Library/Caches/Miuchiz Reborn/<app>/`             | `%LOCALAPPDATA%\Miuchiz Reborn\<app>\` | `~/.cache/miuchiz-reborn/<app>/` |
//! | state   | `~/Library/Application Support/Miuchiz Reborn/<app>/` | `%LOCALAPPDATA%\Miuchiz Reborn\<app>\` | `~/.local/state/miuchiz-reborn/<app>/` |
//! | runtime | `$TMPDIR/Miuchiz Reborn/<app>/`                      | `%TMP%\Miuchiz Reborn\<app>\`          | `$XDG_RUNTIME_DIR/miuchiz-reborn/<app>/` |
//!
//! macOS and Windows map several categories to the same root; apps namespace
//! their own files within. `MIUCHIZ_REBORN_HOME` reroots every category under
//! one directory (`<home>/{config,data,cache,state,runtime}/<app>`) for
//! portable installs and tests.
//!
//! The runtime category is for per-boot rendezvous files (sockets, endpoint
//! files, pids): the OS runtime directory where one exists (Linux
//! `$XDG_RUNTIME_DIR` - per-user, tmpfs, cleared at logout), else the system
//! temp directory. Nothing durable belongs there.
//!
//! This table is a *specification*: implementations in other languages (e.g.
//! libmiuchiz-usb's C endpoint discovery) mirror it, and `test-vectors.txt`
//! in this repository is the shared conformance suite - keep it in sync with
//! any change here.

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
    runtime: PathBuf,
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
        let [config, data, cache, state, runtime] = resolve(&app);
        Self {
            app,
            config,
            data,
            cache,
            state,
            runtime,
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

    /// Per-boot rendezvous files (sockets, endpoint files, pids). May vanish
    /// at logout or reboot; never store anything durable here.
    pub fn runtime_dir(&self) -> &Path {
        &self.runtime
    }
}

/// Create a directory and its parents; idempotent.
pub fn ensure(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

fn resolve(app: &str) -> [PathBuf; 5] {
    if let Some(home) = std::env::var_os(ENV_HOME).filter(|v| !v.is_empty()) {
        return single_root_layout(Path::new(&home), app);
    }
    match BaseDirs::new() {
        Some(base) => {
            let [config, data, cache, state] = os_layout(
                base.config_dir(),
                base.data_dir(),
                base.cache_dir(),
                // macOS/Windows have no dedicated state dir; use local data there.
                base.state_dir().unwrap_or_else(|| base.data_local_dir()),
                app,
            );
            // The OS runtime dir where one exists (Linux $XDG_RUNTIME_DIR),
            // else the system temp dir - both per-boot, which is the point.
            let runtime_root = base
                .runtime_dir()
                .map(Path::to_path_buf)
                .unwrap_or_else(std::env::temp_dir);
            let runtime = runtime_root.join(umbrella()).join(app);
            [config, data, cache, state, runtime]
        }
        None => single_root_layout(Path::new(".miuchiz-reborn"), app),
    }
}

/// `[config, data, cache, state]` as `<root>/<umbrella>/<app>` per OS root.
fn os_layout(config: &Path, data: &Path, cache: &Path, state: &Path, app: &str) -> [PathBuf; 4] {
    let umb = umbrella();
    let leaf = |root: &Path| root.join(umb).join(app);
    [leaf(config), leaf(data), leaf(cache), leaf(state)]
}

/// `[config, data, cache, state, runtime]` as `<home>/<category>/<app>`
/// under one root.
fn single_root_layout(home: &Path, app: &str) -> [PathBuf; 5] {
    let leaf = |cat: &str| home.join(cat).join(app);
    [
        leaf("config"),
        leaf("data"),
        leaf("cache"),
        leaf("state"),
        leaf("runtime"),
    ]
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
        assert_eq!(dirs[4], Path::new("/portable/mr/runtime/launcher"));
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

    /// Runs the shared conformance suite (test-vectors.txt) - the same file
    /// other-language implementations of this policy test against.
    #[test]
    fn conformance_vectors() {
        let vectors = include_str!("../test-vectors.txt");
        let this_platform = if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else {
            "linux"
        };

        let mut ran = 0;
        for line in vectors.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('|').collect();
            assert_eq!(fields.len(), 5, "malformed vector: {line}");
            let [platforms, env, category, app, expected] =
                [fields[0], fields[1], fields[2], fields[3], fields[4]];
            if platforms != "all" && platforms != this_platform {
                continue;
            }

            for assignment in env.split(',') {
                let (key, value) = assignment
                    .split_once('=')
                    .unwrap_or_else(|| panic!("malformed env in vector: {line}"));
                if value.is_empty() {
                    std::env::remove_var(key);
                } else {
                    std::env::set_var(key, value);
                }
            }

            let dirs = AppDirs::new(app);
            let got = match category {
                "config" => dirs.config_dir(),
                "data" => dirs.data_dir(),
                "cache" => dirs.cache_dir(),
                "state" => dirs.state_dir(),
                "runtime" => dirs.runtime_dir(),
                other => panic!("unknown category {other} in vector: {line}"),
            };
            let got = got.to_string_lossy().replace('\\', "/");
            assert_eq!(&got, expected, "vector failed: {line}");
            ran += 1;
        }
        assert!(ran >= 6, "suspiciously few vectors ran ({ran})");
        std::env::remove_var(ENV_HOME);
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
