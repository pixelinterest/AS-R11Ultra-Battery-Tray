//! Rotating file logging under %LOCALAPPDATA%\\R11UltraBattery\\.

use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use tracing_subscriber::fmt::writer::MakeWriter;
use tracing_subscriber::EnvFilter;

const LOG_FILES_TO_KEEP: usize = 3;
const MAX_LOG_FILE_BYTES: u64 = 256 * 1024;

pub fn log_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs_home());
    base.join("R11UltraBattery")
}

pub fn log_file_path() -> PathBuf {
    log_dir().join("tray.log")
}

fn dirs_home() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn init_logging(default_level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(default_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let log_path = log_file_path();
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let can_open = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .is_ok();

    if can_open {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_thread_ids(true)
            .with_ansi(false)
            .with_writer(LogFileWriter {
                path: log_path.clone(),
            })
            .try_init();
        tracing::info!("logging to {}", log_path.display());
    } else {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_thread_ids(true)
            .try_init();
        tracing::warn!(
            "failed to open log file {}, using stderr",
            log_path.display()
        );
    }
}

#[derive(Clone, Debug)]
struct LogFileWriter {
    path: PathBuf,
}

impl<'a> MakeWriter<'a> for LogFileWriter {
    type Writer = Box<dyn io::Write + Send + 'a>;

    fn make_writer(&'a self) -> Self::Writer {
        let _ = maybe_rotate_logs(&self.path, MAX_LOG_FILE_BYTES, LOG_FILES_TO_KEEP);
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            Ok(file) => Box::new(file),
            Err(_) => Box::new(io::sink()),
        }
    }
}

fn maybe_rotate_logs(base: &Path, max_bytes: u64, keep: usize) -> io::Result<()> {
    let size = match fs::metadata(base) {
        Ok(meta) => meta.len(),
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };

    if size < max_bytes || keep <= 1 {
        return Ok(());
    }

    let archive_max = keep - 1;
    let oldest = rotated_path(base, archive_max)?;
    if oldest.exists() {
        fs::remove_file(&oldest)?;
    }
    for idx in (1..archive_max).rev() {
        let src = rotated_path(base, idx)?;
        if src.exists() {
            fs::rename(&src, rotated_path(base, idx + 1)?)?;
        }
    }
    if base.exists() {
        fs::rename(base, rotated_path(base, 1)?)?;
    }
    Ok(())
}

fn rotated_path(base: &Path, index: usize) -> io::Result<PathBuf> {
    let parent = base.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "missing parent directory")
    })?;
    let name = base
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing file name"))?
        .to_string_lossy();
    Ok(parent.join(format!("{name}.{index}")))
}
