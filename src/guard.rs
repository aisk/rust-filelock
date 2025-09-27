use crate::FileLock;

/// A guard that holds a file lock and automatically releases it when dropped.
///
/// This guard is returned by `FileLock::lock()` and ensures that the lock is
/// properly released when it goes out of scope.
#[must_use = "this guard holds a file lock; if not used, the lock will be immediately released"]
pub struct FileLockGuard<'a> {
    lock: &'a mut FileLock,
    unlocked: bool,
}

impl<'a> FileLockGuard<'a> {
    pub fn new(lock: &'a mut FileLock) -> Self {
        FileLockGuard {
            lock,
            unlocked: false,
        }
    }

    /// Manually unlocks the file lock.
    ///
    /// This method consumes the guard and returns a result indicating whether
    /// the unlock operation was successful.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use filelock::FileLock;
    ///
    /// let mut lock = FileLock::new("myfile.lock");
    /// let guard = lock.lock().unwrap();
    ///
    /// // Perform critical operations
    ///
    /// // Manually unlock with error handling
    /// guard.unlock().unwrap();
    /// ```
    pub fn unlock(mut self) -> Result<(), errno::Errno> {
        if !self.unlocked {
            self.lock.unlock()?;
            self.unlocked = true;
        }
        Ok(())
    }
}

impl Drop for FileLockGuard<'_> {
    fn drop(&mut self) {
        if !self.unlocked {
            let _ = self.lock.unlock();
        }
    }
}
