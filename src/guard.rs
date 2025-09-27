use crate::FileLock;

#[must_use = "this guard holds a file lock; if not used, the lock will be immediately released"]
pub struct FileLockGuard<'a> {
    lock: &'a mut FileLock,
}

impl<'a> FileLockGuard<'a> {
    pub fn new(lock: &'a mut FileLock) -> Self {
        FileLockGuard { lock }
    }
}

impl Drop for FileLockGuard<'_> {
    fn drop(&mut self) {
        let _ = self.lock.unlock();
    }
}
