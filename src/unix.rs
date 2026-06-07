extern crate errno;
extern crate libc;

use crate::FileLockGuard;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

pub struct FileLock {
    filename: PathBuf,
    fd: libc::c_int,
}

impl FileLock {
    pub fn new<P: AsRef<Path>>(filename: P) -> FileLock {
        FileLock {
            filename: filename.as_ref().to_path_buf(),
            fd: -1,
        }
    }

    pub fn lock(&mut self) -> Result<FileLockGuard<'_>, errno::Errno> {
        unsafe {
            let c_filename = CString::new(self.filename.as_os_str().as_bytes()).unwrap();
            let fd = libc::open(c_filename.as_ptr(), libc::O_RDWR | libc::O_CREAT, 0o644);
            if fd < 0 {
                return Err(errno::errno());
            }
            self.fd = fd;

            if libc::flock(fd, libc::LOCK_EX) != 0 {
                return Err(errno::errno());
            }
            Ok(FileLockGuard::new(self))
        }
    }

    pub(crate) fn unlock(&mut self) -> Result<(), errno::Errno> {
        let fd = self.fd;
        if fd < 0 {
            return Ok(());
        }
        // Reset the descriptor first so that, whatever happens below, this lock
        // is never closed twice (e.g. by a later Drop).
        self.fd = -1;

        unsafe {
            // close() releases any flock held on the descriptor, so we always
            // close it and only then report the first error encountered.
            let unlock_failed = libc::flock(fd, libc::LOCK_UN) != 0;
            let unlock_errno = errno::errno();

            if libc::close(fd) != 0 {
                return Err(errno::errno());
            }
            if unlock_failed {
                return Err(unlock_errno);
            }
        }

        Ok(())
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        if self.fd >= 0 {
            unsafe {
                libc::close(self.fd);
            }
            self.fd = -1;
        }

        // The lock file is intentionally left on disk. flock locks are bound to
        // the file's inode, so deleting the file would let a concurrent process
        // create a fresh inode and lock it independently, breaking mutual
        // exclusion.
    }
}
