use crate::{Error, ErrorOperation, FileLockGuard, Result};
use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;

/// A handle to a lock file.
///
/// The lock file is opened (and created if missing) by [`FileLock::new`] and
/// stays open for the lifetime of this value. Locks are acquired and released
/// on this open file, so repeated lock/unlock cycles reuse the same file
/// descriptor or handle. Dropping a `FileLock` closes the file, which also
/// releases any lock still held on it.
#[derive(Debug)]
pub struct FileLock {
    file: File,
}

impl FileLock {
    /// Opens (creating if necessary) the lock file at `filename`.
    ///
    /// On Unix the file is created with mode `0644` before applying the
    /// process umask.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use filelock::FileLock;
    ///
    /// let mut lock = FileLock::new("myfile.lock")?;
    /// let _guard = lock.lock()?;
    /// # Ok::<(), filelock::Error>(())
    /// ```
    pub fn new<P: AsRef<Path>>(filename: P) -> Result<FileLock> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o644);
        }
        let file = options
            .open(filename)
            .map_err(|error| Error::new(ErrorOperation::Open, error))?;
        Ok(FileLock { file })
    }

    /// Wraps an already-open file so it can be locked through this API.
    ///
    /// This allows customizing how the lock file is opened (permissions,
    /// open options, pre-existing temporary files, and so on). On Windows the
    /// file must be opened with read or write access for locking to succeed.
    pub fn from_file(file: File) -> FileLock {
        FileLock { file }
    }

    /// Returns a reference to the underlying lock file.
    ///
    /// This can be used to read or write auxiliary data stored in the lock
    /// file, such as the holder's process id.
    pub fn file(&self) -> &File {
        &self.file
    }

    /// Consumes this value and returns the underlying file.
    ///
    /// Any lock held on the file remains associated with the returned handle
    /// and is released when it is closed.
    pub fn into_file(self) -> File {
        self.file
    }

    /// Acquires an exclusive lock, blocking until it becomes available.
    ///
    /// The returned guard releases the lock when dropped.
    pub fn lock(&mut self) -> Result<FileLockGuard<'_>> {
        loop {
            match self.file.lock() {
                Ok(()) => return Ok(FileLockGuard::new(self)),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(Error::new(ErrorOperation::Lock, error)),
            }
        }
    }

    /// Acquires a shared lock, blocking until it becomes available.
    ///
    /// Multiple `FileLock` instances, in this or other processes, may hold a
    /// shared lock on the same file at the same time, but no exclusive lock
    /// can be held concurrently.
    pub fn lock_shared(&mut self) -> Result<FileLockGuard<'_>> {
        loop {
            match self.file.lock_shared() {
                Ok(()) => return Ok(FileLockGuard::new(self)),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(Error::new(ErrorOperation::Lock, error)),
            }
        }
    }

    /// Attempts to acquire an exclusive lock without blocking.
    ///
    /// Returns `Ok(None)` when another process or thread currently holds a
    /// conflicting lock, and `Ok(Some(_))` when the lock was acquired.
    pub fn try_lock(&mut self) -> Result<Option<FileLockGuard<'_>>> {
        if self.try_acquire(File::try_lock)? {
            Ok(Some(FileLockGuard::new(self)))
        } else {
            Ok(None)
        }
    }

    /// Attempts to acquire a shared lock without blocking.
    ///
    /// Returns `Ok(None)` when an exclusive lock is currently held elsewhere,
    /// and `Ok(Some(_))` when the lock was acquired.
    pub fn try_lock_shared(&mut self) -> Result<Option<FileLockGuard<'_>>> {
        if self.try_acquire(File::try_lock_shared)? {
            Ok(Some(FileLockGuard::new(self)))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn try_acquire(
        &self,
        try_lock: fn(&File) -> std::result::Result<(), TryLockError>,
    ) -> Result<bool> {
        loop {
            match try_lock(&self.file) {
                Ok(()) => return Ok(true),
                Err(TryLockError::WouldBlock) => return Ok(false),
                Err(TryLockError::Error(error)) if error.kind() == io::ErrorKind::Interrupted => {
                    continue;
                }
                Err(TryLockError::Error(error)) => {
                    return Err(Error::new(ErrorOperation::Lock, error));
                }
            }
        }
    }

    pub(crate) fn unlock(&mut self) -> Result<()> {
        self.file
            .unlock()
            .map_err(|error| Error::new(ErrorOperation::Unlock, error))
    }
}
