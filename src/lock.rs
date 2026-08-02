use crate::sys::{self, TryLockError};
use crate::{FileLockGuard, OwnedFileLockGuard};
use std::fs::{File, OpenOptions};
use std::io::{self, Result};
use std::path::Path;
use std::time::{Duration, Instant};

pub(crate) const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(1);
pub(crate) const MAX_RETRY_DELAY: Duration = Duration::from_millis(50);

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
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn new<P: AsRef<Path>>(filename: P) -> Result<FileLock> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o644);
        }
        Ok(FileLock {
            file: options.open(filename)?,
        })
    }

    /// Wraps an already-open file so it can be locked through this API.
    ///
    /// This allows customizing how the lock file is opened (permissions,
    /// open options, pre-existing temporary files, and so on). On Windows the
    /// file must be opened with read or write access for locking to succeed;
    /// handles opened for overlapped I/O are supported.
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
        self.blocking_acquire(sys::lock)?;
        Ok(FileLockGuard::new(self))
    }

    /// Acquires a shared lock, blocking until it becomes available.
    ///
    /// Multiple `FileLock` instances, in this or other processes, may hold a
    /// shared lock on the same file at the same time, but no exclusive lock
    /// can be held concurrently.
    pub fn lock_shared(&mut self) -> Result<FileLockGuard<'_>> {
        self.blocking_acquire(sys::lock_shared)?;
        Ok(FileLockGuard::new(self))
    }

    /// Acquires an exclusive lock, blocking until it becomes available, and
    /// returns a guard that owns this `FileLock`.
    ///
    /// Unlike [`lock`](FileLock::lock), the returned guard does not borrow
    /// this value, so it can be stored in long-lived structures or moved
    /// across threads. [`OwnedFileLockGuard::unlock`] returns the `FileLock`
    /// for reuse.
    ///
    /// There are no owned variants of [`try_lock`](FileLock::try_lock) and
    /// [`lock_timeout`](FileLock::lock_timeout), because on contention they
    /// could not return both the failure and the consumed `FileLock`. Use the
    /// borrowing variants for conditional acquisition, or reopen with
    /// [`new`](FileLock::new).
    pub fn lock_owned(self) -> Result<OwnedFileLockGuard> {
        self.blocking_acquire(sys::lock)?;
        Ok(OwnedFileLockGuard::new(self))
    }

    /// Acquires a shared lock, blocking until it becomes available, and
    /// returns a guard that owns this `FileLock`.
    ///
    /// See [`lock_owned`](FileLock::lock_owned).
    pub fn lock_shared_owned(self) -> Result<OwnedFileLockGuard> {
        self.blocking_acquire(sys::lock_shared)?;
        Ok(OwnedFileLockGuard::new(self))
    }

    /// Acquires an exclusive lock, waiting at most `timeout`.
    ///
    /// Returns `Ok(None)` when the lock could not be acquired within the
    /// timeout, and `Ok(Some(_))` when the lock was acquired. A zero timeout
    /// behaves like [`try_lock`](FileLock::try_lock).
    ///
    /// Waiting is implemented by polling the platform's non-blocking lock
    /// operation with bounded exponential backoff, so the actual wait may
    /// exceed `timeout` by up to one backoff interval (at most 50ms) plus
    /// scheduling delays. Polling also means waiters are not queued fairly:
    /// under sustained contention there is no FIFO ordering among waiters,
    /// unlike the blocking [`lock`](FileLock::lock), which waits in the
    /// operating system's lock queue.
    pub fn lock_timeout(&mut self, timeout: Duration) -> Result<Option<FileLockGuard<'_>>> {
        if self.wait_timeout(sys::try_lock, timeout)? {
            Ok(Some(FileLockGuard::new(self)))
        } else {
            Ok(None)
        }
    }

    /// Acquires a shared lock, waiting at most `timeout`.
    ///
    /// See [`lock_timeout`](FileLock::lock_timeout).
    pub fn lock_shared_timeout(&mut self, timeout: Duration) -> Result<Option<FileLockGuard<'_>>> {
        if self.wait_timeout(sys::try_lock_shared, timeout)? {
            Ok(Some(FileLockGuard::new(self)))
        } else {
            Ok(None)
        }
    }

    /// Attempts to acquire an exclusive lock without blocking.
    ///
    /// Returns `Ok(None)` when another process or thread currently holds a
    /// conflicting lock, and `Ok(Some(_))` when the lock was acquired.
    pub fn try_lock(&mut self) -> Result<Option<FileLockGuard<'_>>> {
        if self.try_acquire(sys::try_lock)? {
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
        if self.try_acquire(sys::try_lock_shared)? {
            Ok(Some(FileLockGuard::new(self)))
        } else {
            Ok(None)
        }
    }

    fn blocking_acquire(&self, lock: fn(&File) -> io::Result<()>) -> Result<()> {
        loop {
            match lock(&self.file) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                result => return result,
            }
        }
    }

    fn wait_timeout(
        &self,
        try_lock: fn(&File) -> std::result::Result<(), TryLockError>,
        timeout: Duration,
    ) -> Result<bool> {
        // A timeout too large to represent as a deadline waits unboundedly.
        let deadline = Instant::now().checked_add(timeout);
        let mut retry_delay = INITIAL_RETRY_DELAY;

        loop {
            if self.try_acquire(try_lock)? {
                return Ok(true);
            }

            let mut sleep_for = retry_delay;
            if let Some(deadline) = deadline {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Ok(false);
                }
                sleep_for = sleep_for.min(remaining);
            }
            std::thread::sleep(sleep_for);
            retry_delay = retry_delay.saturating_mul(2).min(MAX_RETRY_DELAY);
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
                Err(TryLockError::Error(error)) => return Err(error),
            }
        }
    }

    pub(crate) fn unlock(&mut self) -> Result<()> {
        sys::unlock(&self.file)
    }
}
