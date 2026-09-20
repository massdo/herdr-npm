use std::fs::{File, OpenOptions};
use std::path::Path;

use crate::domain::error::AppError;

/// Exclusive OS lock on `launcher.lock`. Released when this guard is dropped,
/// including on process death.
pub struct LauncherLock {
    _file: File,
}

/// Wait for an exclusive lock under `HERDR_PLUGIN_STATE_DIR` (or the XDG
/// fallback already resolved by `env::load`).
pub fn acquire(state_dir: &Path) -> Result<LauncherLock, AppError> {
    std::fs::create_dir_all(state_dir)?;
    let path = state_dir.join("launcher.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)?;
    file.lock().map_err(|error| AppError::Io {
        message: format!("cannot acquire {}: {error}", path.display()),
    })?;
    Ok(LauncherLock { _file: file })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn second_acquirer_waits_until_the_first_drops() {
        let dir = std::env::temp_dir().join(format!("herdr-npm-lock-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let held = acquire(&dir).unwrap();
        let dir_clone = dir.clone();
        let barrier_clone = barrier.clone();
        let started = Instant::now();
        let handle = thread::spawn(move || {
            barrier_clone.wait();
            let _second = acquire(&dir_clone).unwrap();
            Instant::now()
        });
        thread::sleep(Duration::from_millis(80));
        barrier.wait();
        drop(held);
        let second_at = handle.join().unwrap();
        assert!(
            second_at.duration_since(started) >= Duration::from_millis(70),
            "second lock was not serialised"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
