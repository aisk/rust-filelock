#![doc = include_str!("../README.md")]

mod guard;
pub use guard::{FileLockGuard, OwnedFileLockGuard};

mod sys;

mod lock;
pub use lock::FileLock;

#[cfg(feature = "tokio")]
mod tokio;

use std::path::Path;

/// Opens (creating if necessary) the lock file at `filename`.
///
/// This is a convenience function equivalent to `FileLock::new(filename)`.
///
/// # Examples
///
/// ```no_run
/// use filelock;
///
/// let mut lock = filelock::new("myfile.lock").unwrap();
/// let _guard = lock.lock().unwrap();
/// ```
pub fn new<P: AsRef<Path>>(filename: P) -> std::io::Result<FileLock> {
    FileLock::new(filename)
}

#[cfg(test)]
mod tests {
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn it_works() {
        let filename =
            std::env::temp_dir().join(format!("filelock-unit-{}.lock", std::process::id()));
        {
            let mut lock = super::new(&filename).unwrap();
            let _guard = lock.lock().unwrap();
        }
        std::fs::remove_file(filename).unwrap();
    }

    #[test]
    fn file_lock_is_send_and_sync() {
        assert_send_sync::<super::FileLock>();
    }
}
