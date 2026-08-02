use crate::lock::{INITIAL_RETRY_DELAY, MAX_RETRY_DELAY};
use crate::{FileLock, FileLockGuard, OwnedFileLockGuard, Result};
use std::fs::File;

impl FileLock {
    /// Acquires an exclusive lock, waiting asynchronously while it is held
    /// elsewhere.
    ///
    /// This method attempts the platform's non-blocking lock operation and
    /// asynchronously waits with bounded exponential backoff while the lock is
    /// unavailable. Lock contention therefore does not park a Tokio runtime
    /// worker. Dropping the returned future leaves no background lock operation
    /// behind.
    ///
    /// This method requires the `tokio` crate feature and must be called from a
    /// Tokio runtime with time enabled.
    ///
    /// # Panics
    ///
    /// Panics if it must wait and is called outside a Tokio runtime, or if the
    /// current runtime does not have time enabled.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "tokio")]
    /// # async fn example() -> Result<(), filelock::Error> {
    /// let mut lock = filelock::FileLock::new("myfile.lock")?;
    /// let _guard = lock.lock_async().await?;
    ///
    /// // Perform async critical operations
    /// # Ok(())
    /// # }
    /// ```
    pub async fn lock_async(&mut self) -> Result<FileLockGuard<'_>> {
        self.wait_async(File::try_lock).await?;
        Ok(FileLockGuard::new(self))
    }

    /// Acquires a shared lock, waiting asynchronously while an exclusive lock
    /// is held elsewhere.
    ///
    /// See [`lock_async`](FileLock::lock_async) for runtime requirements and
    /// cancellation behavior.
    pub async fn lock_shared_async(&mut self) -> Result<FileLockGuard<'_>> {
        self.wait_async(File::try_lock_shared).await?;
        Ok(FileLockGuard::new(self))
    }

    /// Acquires an exclusive lock asynchronously and returns a guard that owns
    /// this `FileLock`.
    ///
    /// The returned guard has no lifetime, so it can be moved into spawned
    /// tasks or stored in long-lived structures. [`OwnedFileLockGuard::unlock`]
    /// returns the `FileLock` for reuse.
    ///
    /// See [`lock_async`](FileLock::lock_async) for runtime requirements and
    /// cancellation behavior.
    pub async fn lock_owned_async(self) -> Result<OwnedFileLockGuard> {
        self.wait_async(File::try_lock).await?;
        Ok(OwnedFileLockGuard::new(self))
    }

    /// Acquires a shared lock asynchronously and returns a guard that owns
    /// this `FileLock`.
    ///
    /// See [`lock_owned_async`](FileLock::lock_owned_async).
    pub async fn lock_shared_owned_async(self) -> Result<OwnedFileLockGuard> {
        self.wait_async(File::try_lock_shared).await?;
        Ok(OwnedFileLockGuard::new(self))
    }

    async fn wait_async(
        &self,
        try_lock: fn(&File) -> std::result::Result<(), std::fs::TryLockError>,
    ) -> Result<()> {
        let mut retry_delay = INITIAL_RETRY_DELAY;

        loop {
            if self.try_acquire(try_lock)? {
                return Ok(());
            }

            ::tokio::time::sleep(retry_delay).await;
            retry_delay = retry_delay.saturating_mul(2).min(MAX_RETRY_DELAY);
        }
    }
}
