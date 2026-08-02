use crate::{FileLock, FileLockGuard, Result};
use std::time::Duration;

const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(1);
const MAX_RETRY_DELAY: Duration = Duration::from_millis(50);

impl FileLock {
    /// Acquires the lock, waiting asynchronously while it is held elsewhere.
    ///
    /// This method attempts the platform's non-blocking lock operation and
    /// asynchronously waits with bounded exponential backoff while the lock is
    /// unavailable. Lock contention therefore does not park a Tokio runtime
    /// worker. Dropping the returned future leaves no background lock operation
    /// behind.
    ///
    /// Each attempt still opens the lock file using an ordinary synchronous
    /// filesystem call. Opening or resolving a path may itself block, especially
    /// on a slow or remote filesystem, and cancellation takes effect after any
    /// such in-progress call returns.
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
    /// let mut lock = filelock::FileLock::new("myfile.lock");
    /// let _guard = lock.lock_async().await?;
    ///
    /// // Perform async critical operations
    /// # Ok(())
    /// # }
    /// ```
    pub async fn lock_async(&mut self) -> Result<FileLockGuard<'_>> {
        let mut retry_delay = INITIAL_RETRY_DELAY;

        loop {
            if self.try_lock_inner()? {
                return Ok(FileLockGuard::new(self));
            }

            ::tokio::time::sleep(retry_delay).await;
            retry_delay = retry_delay.saturating_mul(2).min(MAX_RETRY_DELAY);
        }
    }
}
