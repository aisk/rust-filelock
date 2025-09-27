use crate::FileLock;

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
