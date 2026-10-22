UniKey v__VERSION__
=================================

Thank you for downloading UniKey. This folder is fully self-contained and
portable: copy it to any __PLATFORM__ machine and run the daemon binary —
no installation, no Rust, no command line.

QUICK START
-----------
1. Double-click `daemon` (or `daemon.exe` on Windows).
   The daemon starts silently and keeps its state inside this folder.
2. Open a browser and visit  http://127.0.0.1:45895/api/v1/health
   to confirm it is running (you should see JSON with "status":"running").

WHAT'S INSIDE
-------------
  daemon / daemon.exe   The UniKey daemon (self-contained binary)
  config/               Editable configuration (unikey.toml) + device state
  logs/                 latest.log, rotated backups, panic.log (crash reports)
  assets/               Icons, fonts, themes and future localisation
  README.txt            This file
  LICENSE               MIT license text
  version.json          Build metadata (version, commit, date, platform)

CONFIGURATION
-------------
Edit config/unikey.toml and restart the daemon. If the file is missing or
broken, UniKey recreates defaults automatically and never crashes.
Ports must be unique. Set a port to 0 to disable that listener.

LOGS & CRASHES
--------------
Logs are written to logs/latest.log and rotate automatically. If the daemon
ever crashes, logs/panic.log records the reason.

UNINSTALL
---------
Delete this folder. There is nothing else to remove — UniKey does not write
outside this folder while unikey.portable is present.

This build: v__VERSION__ (__PLATFORM__) · commit __COMMIT__ · __BUILD_DATE__
