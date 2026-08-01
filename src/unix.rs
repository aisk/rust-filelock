extern crate libc;

use crate::{Error, ErrorOperation, FileLockGuard, Result};
use std::ffi::CString;
use std::io;
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

    pub fn lock(&mut self) -> Result<FileLockGuard<'_>> {
        unsafe {
            let c_filename = CString::new(self.filename.as_os_str().as_bytes()).map_err(|_| {
                Error::new(
                    ErrorOperation::Open,
                    io::Error::from(io::ErrorKind::InvalidInput),
                )
            })?;
            let fd = libc::open(
                c_filename.as_ptr(),
                libc::O_RDWR | libc::O_CREAT | libc::O_CLOEXEC,
                0o644,
            );
            if fd < 0 {
                return Err(Error::new(ErrorOperation::Open, io::Error::last_os_error()));
            }

            loop {
                if libc::flock(fd, libc::LOCK_EX) == 0 {
                    break;
                }

                let lock_error = io::Error::last_os_error();
                if lock_error.kind() == io::ErrorKind::Interrupted {
                    continue;
                }

                libc::close(fd);
                return Err(Error::new(ErrorOperation::Lock, lock_error));
            }
            self.fd = fd;
            Ok(FileLockGuard::new(self))
        }
    }

    /// Attempts to acquire the lock without blocking.
    ///
    /// Returns `Ok(None)` when another process or thread currently holds the
    /// lock, and `Ok(Some(_))` when the lock was acquired.
    pub fn try_lock(&mut self) -> Result<Option<FileLockGuard<'_>>> {
        unsafe {
            let c_filename = CString::new(self.filename.as_os_str().as_bytes()).map_err(|_| {
                Error::new(
                    ErrorOperation::Open,
                    io::Error::from(io::ErrorKind::InvalidInput),
                )
            })?;
            let fd = libc::open(
                c_filename.as_ptr(),
                libc::O_RDWR | libc::O_CREAT | libc::O_CLOEXEC,
                0o644,
            );
            if fd < 0 {
                return Err(Error::new(ErrorOperation::Open, io::Error::last_os_error()));
            }

            if libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) != 0 {
                let lock_error = io::Error::last_os_error();
                libc::close(fd);
                if matches!(lock_error.raw_os_error(), Some(code) if code == libc::EWOULDBLOCK || code == libc::EAGAIN)
                {
                    return Ok(None);
                }
                return Err(Error::new(ErrorOperation::Lock, lock_error));
            }

            self.fd = fd;
            Ok(Some(FileLockGuard::new(self)))
        }
    }

    pub(crate) fn unlock(&mut self) -> Result<()> {
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
            let unlock_error = io::Error::last_os_error();
            let close_failed = libc::close(fd) != 0;
            let close_error = io::Error::last_os_error();

            if unlock_failed {
                return Err(Error::new(ErrorOperation::Unlock, unlock_error));
            }
            if close_failed {
                return Err(Error::new(ErrorOperation::Close, close_error));
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
