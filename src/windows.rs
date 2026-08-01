extern crate winapi;

use crate::{Error, ErrorOperation, FileLockGuard, Result};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

pub struct FileLock {
    handle: winapi::um::winnt::HANDLE,
    create_error: Option<CreateError>,
}

#[derive(Clone, Copy)]
enum CreateError {
    InvalidInput,
    Os(i32),
}

impl CreateError {
    fn to_io_error(self) -> io::Error {
        match self {
            Self::InvalidInput => io::Error::from(io::ErrorKind::InvalidInput),
            Self::Os(code) => io::Error::from_raw_os_error(code),
        }
    }
}

impl FileLock {
    pub fn new<P: AsRef<Path>>(filename: P) -> FileLock {
        let path = filename.as_ref();

        let mut wide_path: Vec<u16> = path.as_os_str().encode_wide().collect();
        if wide_path.contains(&0) {
            return FileLock {
                handle: winapi::um::handleapi::INVALID_HANDLE_VALUE,
                create_error: Some(CreateError::InvalidInput),
            };
        }
        wide_path.push(0);

        let handle = unsafe {
            winapi::um::fileapi::CreateFileW(
                wide_path.as_ptr(),
                winapi::um::winnt::GENERIC_READ | winapi::um::winnt::GENERIC_WRITE,
                winapi::um::winnt::FILE_SHARE_READ | winapi::um::winnt::FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                winapi::um::fileapi::OPEN_ALWAYS,
                winapi::um::winnt::FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };

        // Save GetLastError immediately if CreateFileW failed, before another
        // Win32 call can overwrite the thread-local value.
        let create_error = if handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            Some(CreateError::Os(
                io::Error::last_os_error()
                    .raw_os_error()
                    .unwrap_or(winapi::shared::winerror::ERROR_GEN_FAILURE as i32),
            ))
        } else {
            None
        };

        FileLock {
            handle,
            create_error,
        }
    }

    pub fn lock(&mut self) -> Result<FileLockGuard<'_>> {
        // Check if file handle is valid from new()
        if self.handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            let error = self
                .create_error
                .map(CreateError::to_io_error)
                .unwrap_or_else(io::Error::last_os_error);
            return Err(Error::new(ErrorOperation::Open, error));
        }

        #[allow(dangling_pointers_from_temporaries)]
        unsafe {
            let mut overlapped: winapi::um::minwinbase::OVERLAPPED = winapi::_core::mem::zeroed();
            let locked = winapi::um::fileapi::LockFileEx(
                self.handle,
                winapi::um::minwinbase::LOCKFILE_EXCLUSIVE_LOCK,
                0,
                !0,
                !0,
                &mut overlapped as *mut winapi::um::minwinbase::OVERLAPPED,
            );

            if locked != winapi::shared::minwindef::TRUE {
                return Err(Error::new(ErrorOperation::Lock, io::Error::last_os_error()));
            }
        }
        Ok(FileLockGuard::new(self))
    }

    /// Attempts to acquire the lock without blocking.
    ///
    /// Returns `Ok(None)` when another process or thread currently holds the
    /// lock, and `Ok(Some(_))` when the lock was acquired.
    pub fn try_lock(&mut self) -> Result<Option<FileLockGuard<'_>>> {
        if self.handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            let error = self
                .create_error
                .map(CreateError::to_io_error)
                .unwrap_or_else(io::Error::last_os_error);
            return Err(Error::new(ErrorOperation::Open, error));
        }

        unsafe {
            let mut overlapped: winapi::um::minwinbase::OVERLAPPED = winapi::_core::mem::zeroed();
            let locked = winapi::um::fileapi::LockFileEx(
                self.handle,
                winapi::um::minwinbase::LOCKFILE_EXCLUSIVE_LOCK
                    | winapi::um::minwinbase::LOCKFILE_FAIL_IMMEDIATELY,
                0,
                !0,
                !0,
                &mut overlapped,
            );

            if locked != winapi::shared::minwindef::TRUE {
                let lock_error = io::Error::last_os_error();
                if lock_error.raw_os_error()
                    == Some(winapi::shared::winerror::ERROR_LOCK_VIOLATION as i32)
                {
                    return Ok(None);
                }
                return Err(Error::new(ErrorOperation::Lock, lock_error));
            }
        }

        Ok(Some(FileLockGuard::new(self)))
    }

    pub(crate) fn unlock(&mut self) -> Result<()> {
        unsafe {
            let mut overlapped: winapi::um::minwinbase::OVERLAPPED = winapi::_core::mem::zeroed();
            let unlocked = winapi::um::fileapi::UnlockFileEx(
                self.handle,
                0,
                !0,
                !0,
                &mut overlapped as *mut winapi::um::minwinbase::OVERLAPPED,
            );

            if unlocked != winapi::shared::minwindef::TRUE {
                return Err(Error::new(
                    ErrorOperation::Unlock,
                    io::Error::last_os_error(),
                ));
            }

            // Don't close handle here - it will be closed when FileLock is dropped
        }

        Ok(())
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        if self.handle != winapi::shared::ntdef::NULL
            && self.handle != winapi::um::handleapi::INVALID_HANDLE_VALUE
        {
            unsafe {
                winapi::um::handleapi::CloseHandle(self.handle);
            }
        }

        // The lock file is intentionally left on disk; deleting it on release
        // would open a window for concurrent processes to bypass the lock.
    }
}
