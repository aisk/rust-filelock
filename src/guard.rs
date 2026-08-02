use crate::{FileLock, Result};

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
    pub(crate) fn new(lock: &'a mut FileLock) -> Self {
        FileLockGuard {
            lock,
            unlocked: false,
        }
    }

    /// Returns a reference to the underlying lock file.
    ///
    /// This can be used to read or write auxiliary data stored in the lock
    /// file, such as the holder's process id, while the lock is held.
    pub fn file(&self) -> &std::fs::File {
        self.lock.file()
    }

    /// Manually unlocks the file lock.
    ///
    /// This method consumes the guard and returns a result indicating whether
    /// the unlock operation was successful.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use filelock::FileLock;
    ///
    /// let mut lock = FileLock::new("myfile.lock").unwrap();
    /// let guard = lock.lock().unwrap();
    ///
    /// // Perform critical operations
    ///
    /// // Manually unlock with error handling
    /// guard.unlock().unwrap();
    /// ```
    pub fn unlock(mut self) -> Result<()> {
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

/// A guard that owns its [`FileLock`] and releases the lock when dropped.
///
/// Unlike [`FileLockGuard`], this guard does not borrow the `FileLock`, so it
/// has no lifetime and can be stored in long-lived structures or moved across
/// threads and tasks. It is returned by `FileLock::lock_owned` and related
/// methods.
#[must_use = "this guard holds a file lock; if not used, the lock will be immediately released"]
pub struct OwnedFileLockGuard {
    lock: Option<FileLock>,
}

impl OwnedFileLockGuard {
    pub(crate) fn new(lock: FileLock) -> Self {
        OwnedFileLockGuard { lock: Some(lock) }
    }

    /// Returns a reference to the underlying lock file.
    ///
    /// This can be used to read or write auxiliary data stored in the lock
    /// file, such as the holder's process id, while the lock is held.
    pub fn file(&self) -> &std::fs::File {
        self.lock.as_ref().unwrap().file()
    }

    /// Manually unlocks the file lock, returning the [`FileLock`] for reuse.
    ///
    /// If releasing the lock fails, the `FileLock` is dropped, which closes
    /// the lock file and thereby releases the lock anyway.
    pub fn unlock(mut self) -> Result<FileLock> {
        let mut lock = self.lock.take().unwrap();
        lock.unlock()?;
        Ok(lock)
    }
}

impl Drop for OwnedFileLockGuard {
    fn drop(&mut self) {
        if let Some(lock) = &mut self.lock {
            let _ = lock.unlock();
        }
    }
}
