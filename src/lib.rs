//! Simple filelock library for rust, using `flock` on Unix-like systems and `LockFileEx` on Windows under the hood.
//!
//! ## Platform behavior
//!
//! All participants must use this locking protocol and resolve the same stable
//! lock-file path. The lock file must not be deleted, renamed, or replaced while
//! participants may be running.
//!
//! Unix locks are advisory and associated with the opened file, not its path.
//! Do not `fork` while holding a guard and then let both parent and child continue
//! through the protected critical section. Lock files are opened read-write and
//! created with mode `0644` before applying the process umask, so cross-user use
//! requires permissions to be arranged explicitly.
//!
//! Windows uses an exclusive byte-range lock. It prevents ordinary reads and
//! writes to that range from other processes, but not access through memory-mapped
//! views. On network filesystems, locking behavior can additionally depend on the
//! client, server, filesystem, and mount configuration.
//!
//! ## Usage
//!
//! ```no_run
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut lock = filelock::new("myfile.lock");
//!     let _guard = lock.lock()?;
//!
//!     // Perform critical operations
//!
//!     // Lock is automatically released when _guard goes out of scope
//!     Ok(())
//! }
//! ```

//! To attempt locking without waiting:
//!
//! ```no_run
//! let mut lock = filelock::new("myfile.lock");
//! if let Some(_guard) = lock.try_lock()? {
//!     // The lock was acquired.
//! } else {
//!     // The lock is currently held elsewhere.
//! }
//! # Ok::<(), filelock::Error>(())
//! ```
//!
//! For manual control:
//!
//! ```no_run
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut lock = filelock::new("myfile.lock");
//!     let guard = lock.lock()?;
//!
//!     // Perform critical operations
//!
//!     // Manually unlock with error handling
//!     guard.unlock()?;
//!
//!     Ok(())
//! }
//! ```

mod guard;
pub use guard::FileLockGuard;

mod error;
pub use error::{Error, ErrorOperation, Result};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::FileLock;

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::FileLock;

use std::path::Path;

/// Creates a new FileLock instance.
///
/// This is a convenience function equivalent to `FileLock::new(filename)`.
///
/// # Examples
///
/// ```no_run
/// use filelock;
///
/// let mut lock = filelock::new("myfile.lock");
/// let _guard = lock.lock().unwrap();
/// ```
pub fn new<P: AsRef<Path>>(filename: P) -> FileLock {
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
            let mut lock = super::new(&filename);
            let _guard = lock.lock().unwrap();
        }
        std::fs::remove_file(filename).unwrap();
    }

    #[test]
    fn file_lock_is_send_and_sync() {
        assert_send_sync::<super::FileLock>();
    }
}
