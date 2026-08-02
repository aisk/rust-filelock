//! Simple filelock library for Rust, built on the standard library's file
//! locking support (`std::fs::File::lock` and friends, stable since Rust
//! 1.89). The standard library uses `flock` (or `fcntl` where `flock` is
//! unavailable) on Unix-like systems and `LockFileEx` on Windows.
//!
//! ## Platform behavior
//!
//! All participants must use this locking protocol and resolve the same stable
//! lock-file path. The lock file must not be deleted, renamed, or replaced
//! while participants may be running.
//!
//! Locks are advisory and associated with the opened file, not its path.
//! Do not `fork` while holding a guard and then let both parent and child
//! continue through the protected critical section. Lock files are opened
//! read-write and created with mode `0644` before applying the process umask,
//! so cross-user use requires permissions to be arranged explicitly.
//!
//! Interactions between file locks and ordinary reads and writes by
//! non-lockholders are platform specific. On network filesystems, locking
//! behavior can additionally depend on the client, server, filesystem, and
//! mount configuration.
//!
//! ## Usage
//!
//! ```no_run
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut lock = filelock::new("myfile.lock")?;
//!     let _guard = lock.lock()?;
//!
//!     // Perform critical operations
//!
//!     // Lock is automatically released when _guard goes out of scope
//!     Ok(())
//! }
//! ```
//!
//! To attempt locking without waiting:
//!
//! ```no_run
//! let mut lock = filelock::new("myfile.lock")?;
//! if let Some(_guard) = lock.try_lock()? {
//!     // The lock was acquired.
//! } else {
//!     // The lock is currently held elsewhere.
//! }
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! Shared (read) locks allow multiple holders at once, while still excluding
//! exclusive locks:
//!
//! ```no_run
//! let mut lock = filelock::new("myfile.lock")?;
//! let _guard = lock.lock_shared()?;
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! To wait for a lock with an upper bound on the wait:
//!
//! ```no_run
//! use std::time::Duration;
//!
//! let mut lock = filelock::new("myfile.lock")?;
//! if let Some(_guard) = lock.lock_timeout(Duration::from_secs(5))? {
//!     // The lock was acquired within five seconds.
//! } else {
//!     // The lock is still held elsewhere.
//! }
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! The guards returned by `lock()` and friends borrow the `FileLock`. When a
//! guard needs to outlive the current scope — stored in a struct or moved into
//! a spawned task — use the owned variants, which take ownership of the
//! `FileLock` and return it on unlock:
//!
//! ```no_run
//! let guard = filelock::new("myfile.lock")?.lock_owned()?;
//! std::thread::spawn(move || {
//!     let _guard = guard;
//!     // Critical section runs on another thread.
//! });
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! With the optional `tokio` feature, a contended lock can be acquired without
//! blocking a Tokio runtime worker on lock contention:
//!
//! ```no_run
//! # #[cfg(feature = "tokio")]
//! # async fn example() -> std::io::Result<()> {
//! let mut lock = filelock::new("myfile.lock")?;
//! let _guard = lock.lock_async().await?;
//!
//! // Perform async critical operations
//! # Ok(())
//! # }
//! ```
//!
//! For manual control:
//!
//! ```no_run
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut lock = filelock::new("myfile.lock")?;
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
pub use guard::{FileLockGuard, OwnedFileLockGuard};

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
