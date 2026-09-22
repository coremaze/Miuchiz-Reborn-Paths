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
//! **Android** is different in kind, and deliberately absent from that table:
//! an app does not *discover* its storage, the system hands it a private root.
//! So there is no `AppDirs::new` there — only [`AppDirs::new_in`], which
//! lays the categories out under the supplied root exactly as
//! `MIUCHIZ_REBORN_HOME` does. No umbrella directory is inserted: the app
//! sandbox already provides the isolation the umbrella provides elsewhere.
//! One wrinkle to know about — `cache` is then a subdirectory of the app's
//! files directory, *not* the OS-evictable `Context.getCacheDir()`, which
//! nothing here can reach without JNI. If eviction semantics start to matter,
//! a `new_in` variant taking a separate cache root is the place to add them.
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

#[cfg(not(target_os = "android"))]
use directories::BaseDirs;

#[cfg(not(target_os = "android"))]
const UMBRELLA_HUMAN: &str = "Miuchiz Reborn";
#[cfg(not(target_os = "android"))]
const UMBRELLA_XDG: &str = "miuchiz-reborn";

/// Reroots every category under one directory (portable installs, tests).
pub const ENV_HOME: &str = "MIUCHIZ_REBORN_HOME";

/// Desktop-only: unreachable on Android, where `AppDirs::new` does not exist.
#[cfg(not(target_os = "android"))]
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
    /// Resolve directories for `app` (a plain slug like `"launcher"`) from the
    /// environment.
    ///
    /// Falls back to a local `./.miuchiz-reborn` tree when there is no home
    /// directory and no [`ENV_HOME`] override.
    ///
    /// **Not available on Android**, where there is no environment-determined
    /// answer: an app's storage root is handed to it by the system at runtime,
    /// and every fallback this function has would resolve outside the app
    /// sandbox and be unwritable — silently, since this function cannot fail.
    /// Use [`AppDirs::new_in`] with the root the platform supplied.
    #[cfg(not(target_os = "android"))]
    pub fn new(app: impl Into<String>) -> Self {
        let app = app.into();
        debug_assert_valid_app(&app);
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

    /// Resolve directories for `app` under one explicit `root`, laid out as
    /// `<root>/{config,data,cache,state,runtime}/<app>` — the same shape
    /// [`ENV_HOME`] produces.
    ///
    /// For platforms that hand an application its storage root rather than
    /// letting it discover one (Android's `Context.getFilesDir()`), and for
    /// portable installs and tests that want a location with no environmental
    /// input at all.
    ///
    /// Deliberately **literal**: unlike `AppDirs::new` this does not consult
    /// [`ENV_HOME`]. An explicitly supplied root is a stronger statement than
    /// an ambient variable, and tests — the other caller — need it
    /// deterministic.
    pub fn new_in(root: impl AsRef<Path>, app: impl Into<String>) -> Self {
        let app = app.into();
        debug_assert_valid_app(&app);
        let [config, data, cache, state, runtime] = single_root_layout(root.as_ref(), &app);
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

fn debug_assert_valid_app(app: &str) {
    debug_assert!(!app.is_empty(), "app name must not be empty");
    debug_assert!(
        !app.contains(['/', '\\']),
        "app name must be a plain slug, not a path"
    );
}

#[cfg(not(target_os = "android"))]
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
#[cfg(not(target_os = "android"))]
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

    /// The public face of the single-root layout, and the only constructor
    /// Android has. Runs everywhere: the layout is one shared rule, so a change
    /// that breaks Android should fail the desktop suite too.
    #[test]
    fn new_in_lays_out_under_the_given_root() {
        let dirs = AppDirs::new_in("/data/user/0/com.example/files", "audoboom");
        let root = Path::new("/data/user/0/com.example/files");
        assert_eq!(dirs.app(), "audoboom");
        assert_eq!(dirs.config_dir(), root.join("config/audoboom"));
        assert_eq!(dirs.data_dir(), root.join("data/audoboom"));
        assert_eq!(dirs.cache_dir(), root.join("cache/audoboom"));
        assert_eq!(dirs.state_dir(), root.join("state/audoboom"));
        assert_eq!(dirs.runtime_dir(), root.join("runtime/audoboom"));
    }

    /// Serialises the tests that mutate process-wide environment variables.
    /// Cargo runs tests on parallel threads and `set_var` is global, so without
    /// this the two below race over [`ENV_HOME`]. Poisoning is recovered from:
    /// a panic in one test should fail that test, not cascade into the other.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// `new_in` is literal: an explicitly supplied root outranks the ambient
    /// override, or tests could not rely on it.
    #[test]
    fn new_in_ignores_env_home() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var(ENV_HOME, "/should/be/ignored");
        let dirs = AppDirs::new_in("/explicit", "audoboom");
        assert_eq!(dirs.config_dir(), Path::new("/explicit/config/audoboom"));
        std::env::remove_var(ENV_HOME);
    }

    #[cfg(not(target_os = "android"))]
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
    ///
    /// Not built on Android: every vector resolves through `AppDirs::new`,
    /// which does not exist there. Android's layout is not unspecified as a
    /// result — it is the single-root layout the `MIUCHIZ_REBORN_HOME` vectors
    /// already pin, exercised by `new_in_lays_out_under_the_given_root` above,
    /// which does run there.
    #[cfg(not(target_os = "android"))]
    #[test]
    fn conformance_vectors() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

    #[cfg(not(target_os = "android"))]
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
