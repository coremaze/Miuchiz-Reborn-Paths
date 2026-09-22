# Miuchiz Reborn Paths

Shared on-disk storage location policy for Miuchiz Reborn applications.

## Layout

One "Miuchiz Reborn" umbrella, a per-app subdirectory inside it, each category in
its OS-correct root:

| Category | macOS | Windows | Linux |
|----------|-------|---------|-------|
| config | `~/Library/Application Support/Miuchiz Reborn/<app>/` | `%APPDATA%\Miuchiz Reborn\<app>\` | `~/.config/miuchiz-reborn/<app>/` |
| data | `~/Library/Application Support/Miuchiz Reborn/<app>/` | `%APPDATA%\Miuchiz Reborn\<app>\` | `~/.local/share/miuchiz-reborn/<app>/` |
| cache | `~/Library/Caches/Miuchiz Reborn/<app>/` | `%LOCALAPPDATA%\Miuchiz Reborn\<app>\` | `~/.cache/miuchiz-reborn/<app>/` |
| state | `~/Library/Application Support/Miuchiz Reborn/<app>/` | `%LOCALAPPDATA%\Miuchiz Reborn\<app>\` | `~/.local/state/miuchiz-reborn/<app>/` |
| runtime | `$TMPDIR/Miuchiz Reborn/<app>/` | `%TMP%\Miuchiz Reborn\<app>\` | `$XDG_RUNTIME_DIR/miuchiz-reborn/<app>/` |

On macOS/Windows the OS maps several categories to the same root by convention;
apps namespace their own files within, so nothing collides.

## Categories

- **config**: user settings; must survive updates. Roams on Windows.
- **data**: durable user data (saves, history); never auto-deleted. Roams on Windows.
- **cache**: regenerable; safe to evict.
- **state**: machine-local bookkeeping and logs.
- **runtime**: per-boot rendezvous files (sockets, endpoint files, pids). The
  OS runtime directory where one exists (Linux `$XDG_RUNTIME_DIR` - per-user,
  tmpfs, cleared at logout), else the system temp directory. May vanish at
  logout or reboot; never store anything durable here.


Config/data/cache are deliberately **shared** across two copies of an app on one
machine, because you want settings and saves to survive moving or reinstalling.

### Overrides

Set `MIUCHIZ_REBORN_HOME` to reroot every category under one directory.
(`<home>/{config,data,cache,state,runtime}/<app>`)

### Android and explicit roots

Android supplies the app-private root at runtime. Use
`AppDirs::new_in(root, "audoboom")` to resolve
`<root>/{config,data,cache,state,runtime}/audoboom`.
This constructor is available on every platform and ignores
`MIUCHIZ_REBORN_HOME`. It resolves paths; callers create directories as needed.

`AppDirs::new` and environment-based discovery are unavailable on Android.
When the supplied root is Android's files directory, the cache subdirectory is
not the OS-evictable `Context.getCacheDir()`.

### Other implementations

This layout is a specification, not just a crate: implementations in other
languages (e.g. libmiuchiz-usb's C endpoint discovery) mirror the parts they
need. `test-vectors.txt` is the shared conformance suite - every
implementation runs it in its tests, so a policy change here either propagates
everywhere or fails loudly.
