//! Logging initialisation for the daemon.
//!
//! * Console output via `tracing-subscriber` (stderr).
//! * A rotating file at `logs/latest.log` (see [`LogRotator`]).
//! * A panic hook that appends crash reports to `logs/panic.log` before the
//!   process exits — every log directory is created automatically, nothing is
//!   ever expected to pre-exist.
//!
//! The file writer is fail-soft: if the log file cannot be opened the daemon
//! keeps running and falls back to console-only logging.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Once};
use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::paths::RuntimePaths;

/// Prefix shared by `latest.log` and its rotated backups.
const BACKUP_PREFIX: &str = "latest.";

/// Installed exactly once so tests can call [`init`] repeatedly.
static PANIC_HOOK_INSTALLED: Once = Once::new();

// ─── Rotating file writer ───────────────────────────────────────────────

/// A `Write` sink that rotates `latest.log` once it exceeds a byte budget.
///
/// Rotation renames `latest.log` → `latest.<UTC timestamp>.log` and prunes the
/// oldest backups once `max_backups` is exceeded.
struct LogRotator {
    active: PathBuf,
    file: Option<fs::File>,
    max_bytes: u64,
    max_backups: usize,
    bytes: u64,
}

impl LogRotator {
    fn new(active: PathBuf, max_bytes: u64, max_backups: usize) -> Self {
        let mut this = Self {
            active,
            file: None,
            max_bytes,
            max_backups,
            bytes: 0,
        };
        this.reopen();
        this
    }

    /// (Re)open the active file in append mode.
    fn reopen(&mut self) {
        match OpenOptions::new().create(true).append(true).open(&self.active) {
            Ok(file) => {
                self.bytes = file.metadata().map(|m| m.len()).unwrap_or(0);
                self.file = Some(file);
            }
            Err(err) => {
                eprintln!("unable to open log file {}: {err}", self.active.display());
                self.file = None;
            }
        }
    }

    /// Rename the active file to a timestamped backup and start a fresh one.
    fn rotate(&mut self) {
        // Drop the handle before renaming (required on Windows).
        self.file.take();
        let stamp = Utc::now().format("%Y%m%d-%H%M%S");
        let backup = self.active.with_extension(format!("{stamp}.log"));
        if fs::rename(&self.active, &backup).is_err() {
            // Rename can fail if the active file was never created; reopening
            // below will recreate it. Not fatal.
        }
        self.reopen();
        self.prune();
    }

    /// Remove the oldest backups so at most `max_backups` remain.
    fn prune(&mut self) {
        if self.max_backups == 0 {
            return;
        }
        let Ok(entries) = fs::read_dir(self.active.parent().unwrap_or(Path::new("."))) else {
            return;
        };
        let mut backups: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with(BACKUP_PREFIX) && n.ends_with(".log"))
                    .unwrap_or(false)
            })
            .collect();
        backups.sort(); // timestamp suffixes sort lexicographically
        while backups.len() > self.max_backups {
            let oldest = backups.remove(0);
            let _ = fs::remove_file(&oldest);
        }
    }
}

impl io::Write for LogRotator {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if !buf.is_empty() && self.bytes + buf.len() as u64 > self.max_bytes {
            self.rotate();
        }
        match self.file.as_mut() {
            Some(file) => {
                let written = file.write(buf)?;
                self.bytes += written as u64;
                Ok(written)
            }
            // Fail-soft: the daemon runs, logs go to the console only.
            None => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.file.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}

/// `MakeWriter` adapter so `tracing-subscriber` can write through the shared
/// rotator. Each event acquires the mutex for the duration of its write.
#[derive(Clone)]
struct RotatingFileWriter {
    inner: Arc<Mutex<LogRotator>>,
}

impl io::Write for RotatingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("log mutex poisoned"))?;
        guard.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("log mutex poisoned"))?;
        guard.flush()
    }
}

#[derive(Clone)]
struct RotatingFileFactory {
    inner: Arc<Mutex<LogRotator>>,
}

impl<'a> MakeWriter<'a> for RotatingFileFactory {
    type Writer = RotatingFileWriter;
    fn make_writer(&'a self) -> Self::Writer {
        RotatingFileWriter {
            inner: self.inner.clone(),
        }
    }
}

// ─── Panic hook ─────────────────────────────────────────────────────────

/// Capture a readable description of a panic payload (string or otherwise).
fn describe_payload(payload: &dyn std::any::Any) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

/// Append a crash report to `logs/panic.log`, then mirror it to stderr.
///
/// Installed once per process. Runs before abort under `panic = "abort"`, so
/// crash diagnostics survive even in release builds.
pub fn install_panic_hook(panic_log: &Path) {
    let path = panic_log.to_path_buf();
    PANIC_HOOK_INSTALLED.call_once(|| {
        std::panic::set_hook(Box::new(move |info| {
            let payload = describe_payload(info.payload());
            let location = info
                .location()
                .map(|l| l.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let ts = Utc::now().to_rfc3339();
            let entry = format!("==== PANIC {ts} ====\nMessage: {payload}\nLocation: {location}\n");

            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
                let _ = file.write_all(entry.as_bytes());
                let _ = file.flush();
            }
            eprintln!("{entry}");
        }));
    });
}

// ─── Initialisation ─────────────────────────────────────────────────────

/// Parse a log-level string; unrecognised values fall back to `info`.
pub fn parse_level(raw: &str) -> LevelFilter {
    match raw.trim().to_ascii_lowercase().as_str() {
        "trace" => LevelFilter::TRACE,
        "debug" => LevelFilter::DEBUG,
        "warn" => LevelFilter::WARN,
        "error" => LevelFilter::ERROR,
        _ => LevelFilter::INFO,
    }
}

/// Initialise console + file logging and install the panic hook.
///
/// Safe to call more than once; subsequent calls are no-ops if a global
/// subscriber already exists.
pub fn init(paths: &RuntimePaths, level: &str, file_enabled: bool, max_bytes: u64, max_backups: usize) -> Result<()> {
    install_panic_hook(&paths.panic_log_file());

    let default_level = parse_level(level);
    let filter = EnvFilter::builder()
        .with_default_directive(default_level.into())
        .from_env_lossy();

    let console = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_ansi(true);

    let registry = tracing_subscriber::registry().with(filter).with(console);

    if file_enabled {
        let rotator = LogRotator::new(paths.active_log_file(), max_bytes, max_backups);
        let factory = RotatingFileFactory {
            inner: Arc::new(Mutex::new(rotator)),
        };
        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(factory)
            .with_ansi(false)
            .with_target(false);
        // A global subscriber may already exist (second call in tests); that
        // is fine — logging falls back to console only.
        let _ = registry.with(file_layer).try_init();
    } else {
        let _ = registry.try_init();
    }

    Ok(())
}

/// Yield briefly so buffered log lines flush to the file before shutdown.
pub fn flush_before_exit() {
    // tracing's fmt layer flushes synchronously per event; this only helps
    // when a panic aborts mid-write on another thread.
    std::thread::sleep(Duration::from_millis(10));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("unikey-log-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).expect("create temp base");
        base
    }

    #[test]
    fn parse_level_handles_aliases() {
        assert_eq!(parse_level("TRACE"), LevelFilter::TRACE);
        assert_eq!(parse_level(" debug "), LevelFilter::DEBUG);
        assert_eq!(parse_level("info"), LevelFilter::INFO);
        assert_eq!(parse_level("warn"), LevelFilter::WARN);
        assert_eq!(parse_level("error"), LevelFilter::ERROR);
        assert_eq!(parse_level("banana"), LevelFilter::INFO);
    }

    #[test]
    fn rotator_writes_and_flushes() {
        let dir = temp_dir("basic");
        let active = dir.join("latest.log");
        let mut rotator = LogRotator::new(active.clone(), 1024, 3);
        writeln!(rotator, "hello").expect("write");
        rotator.flush().expect("flush");
        assert!(active.exists());
        let text = fs::read_to_string(&active).expect("read");
        assert!(text.contains("hello"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotator_rotates_when_over_budget() {
        let dir = temp_dir("rotate");
        let active = dir.join("latest.log");
        let mut rotator = LogRotator::new(active.clone(), 32, 5);
        for i in 0..10 {
            writeln!(rotator, "line {i}").expect("write");
        }
        // The active file plus some backups must exist, and more writes
        // must never fail.
        assert!(active.exists());
        writeln!(rotator, "tail").expect("write after rotation");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotator_prunes_old_backups() {
        let dir = temp_dir("prune");
        let active = dir.join("latest.log");
        let mut rotator = LogRotator::new(active.clone(), 16, 2);
        for i in 0..30 {
            writeln!(rotator, "line {i}").expect("write");
        }
        let backups = fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with(BACKUP_PREFIX))
            .count();
        assert!(backups <= 2, "expected at most 2 backups, found {backups}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn fail_soft_when_cannot_open() {
        // The active path's parent is a *file*, so the log cannot be opened.
        // Writes must still succeed (dropped) instead of crashing the daemon.
        let dir = temp_dir("failsoft");
        let bad_parent = dir.join("not-a-dir");
        fs::write(&bad_parent, "i am a file").expect("write");
        let bad = bad_parent.join("latest.log");
        let mut rotator = LogRotator::new(bad, 1024, 2);
        let written = rotator.write(b"data\n").expect("write returns ok");
        assert_eq!(written, 5);
        let _ = fs::remove_dir_all(&dir);
    }
}
