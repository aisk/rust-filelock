//! Simple filelock library for rust, using `flock` on Unix-like systems and `LockFileEx` on Windows under the hood.
//!
//! ## Usage
//!
//! ```rust
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
//! ```rust
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
//! ```rust
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
/// ```rust
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
        let mut lock = super::new("test.lock");
        let _guard = lock.lock().unwrap();
    }

    #[test]
    fn file_lock_is_send_and_sync() {
        assert_send_sync::<super::FileLock>();
    }
}
