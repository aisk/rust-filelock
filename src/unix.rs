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
            fd: 0,
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

        unsafe {
            if libc::flock(fd, libc::LOCK_UN) != 0 {
                return Err(errno::errno());
            }

            if libc::close(fd) != 0 {
                return Err(errno::errno());
            }
        }

        self.fd = 0;

        Ok(())
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        if self.fd > 0 {
            unsafe {
                libc::close(self.fd);
            }
            self.fd = 0;
        }

        // Try to delete the lock file, allow failure
        let _ = std::fs::remove_file(&self.filename);
    }
}
