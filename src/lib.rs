//! Simple filelock library for rust, using `flock` on Unix-like systems and `LockFileEx` on Windows under the hood.
//!
//! ## Usage
//!
//! ```rust
//! use filelock::FileLock;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut lock = FileLock::new("myfile.lock");
//!     let _guard = lock.lock()?;
//!
//!     // Perform critical operations
//!
//!     // Lock is automatically released when _guard goes out of scope
//!     Ok(())
//! }
//! ```
//!
//! For manual control:
//!
//! ```rust
//! use filelock::FileLock;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut lock = FileLock::new("myfile.lock");
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

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::FileLock;

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::FileLock;

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
pub fn new(filename: &str) -> FileLock {
    return FileLock::new(filename);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let mut lock = super::new("test.lock");
        let _guard = lock.lock().unwrap();
    }
}
