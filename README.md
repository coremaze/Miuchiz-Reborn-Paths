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

On macOS/Windows the OS maps several categories to the same root by convention;
apps namespace their own files within, so nothing collides.

## Categories

- **config**: user settings; must survive updates. Roams on Windows.
- **data**: durable user data (saves, history); never auto-deleted. Roams on Windows.
- **cache**: regenerable; safe to evict.
- **state**: machine-local bookkeeping and logs.


Config/data/cache are deliberately **shared** across two copies of an app on one
machine, because you want settings and saves to survive moving or reinstalling.

### Overrides

Set `MIUCHIZ_REBORN_HOME` to reroot every category under one directory.
(`<home>/{config,data,cache,state}/<app>`)