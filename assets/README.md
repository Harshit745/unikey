# `assets/` — shipped application assets

Read-only resources that are copied into every release folder under
`assets/`. The daemon resolves this directory via `RuntimePaths::assets_dir()`
and does not require it to exist (development builds run without assets).

| Sub-directory | Intended contents                                    |
|---------------|------------------------------------------------------|
| `icons/`      | Application icons (`.ico`, `.icns`, `.png`)          |
| `fonts/`      | Bundled fonts for the desktop UI                     |
| `themes/`     | JSON/TOML theme definitions for future UI theming    |
| `locale/`     | Localisation files (`.ftl`, `.json`, `.po`, …)       |

## Future-proofing

The build scripts copy this entire directory verbatim, so adding a new
resource (or an entirely new asset category) requires **no changes** to the
pipeline. Drop the file in, rebuild, it ships.
