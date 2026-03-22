# Download Page: Link to GitHub Releases

## Summary

Wire the download page buttons to the latest GitHub release assets using the GitHub API at runtime.

## Behavior

On mount, the download page fetches the latest release from the GitHub API:

```
GET https://api.github.com/repos/felixscode/jolly/releases/latest
```

The response `assets` array is matched to buttons by file extension:

| Button | Match pattern |
|--------|--------------|
| Windows .exe | `.exe` |
| macOS .dmg | `.dmg` |
| Linux .AppImage | `.AppImage` |
| Linux .deb | `.deb` |
| Linux .rpm | `.rpm` |

Each button's `href` is set to the matching asset's `browser_download_url`.

The release `tag_name` (e.g. `app-v0.0.4`) is displayed as a version label on the page.

## Loading and error states

- **While fetching:** buttons are disabled (no `href`), showing a brief loading indicator.
- **On failure or missing release:** all buttons fall back to `https://github.com/felixscode/jolly/releases/latest`.

## Scope

- **One file changed:** `src/routes/download/+page.svelte`
- **No changes to:** home page, layout, styling, platform detection
- **No new dependencies**
