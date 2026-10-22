# `config/` — shipped default configuration

| File           | Purpose                                                        |
|----------------|----------------------------------------------------------------|
| `unikey.toml`  | Default daemon configuration. Copied into every release folder |
| `README.md`    | This file (not shipped).                                       |

## How it is used

1. The build scripts copy `unikey.toml` into `release/UniKey/config/`.
2. On first run the daemon loads it; if it is missing or malformed the daemon
   **creates a default and never crashes** (see `runtime/daemon/src/config.rs`).

## Editing

Any value can be changed in a release install and takes effect on the next
daemon restart. Ports must stay unique across `[network]`; setting a port to
`0` disables that listener.

## Future additions

Drop new files in here and they are copied automatically — the build scripts
copy the whole directory, so future localisation, service profiles, or
network presets need no pipeline changes.
