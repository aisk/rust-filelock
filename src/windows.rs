extern crate winapi;

use crate::{Error, ErrorOperation, FileLockGuard, Result};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, IntoRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};

pub struct FileLock {
    filename: PathBuf,
    handle: Option<OwnedHandle>,
}

impl FileLock {
    pub fn new<P: AsRef<Path>>(filename: P) -> FileLock {
        FileLock {
            filename: filename.as_ref().to_path_buf(),
            handle: None,
        }
    }

    fn open(&mut self) -> Result<()> {
        let mut wide_path: Vec<u16> = self.filename.as_os_str().encode_wide().collect();
        if wide_path.contains(&0) {
            return Err(Error::new(
                ErrorOperation::Open,
                io::Error::from(io::ErrorKind::InvalidInput),
            ));
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

        if handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            return Err(Error::new(ErrorOperation::Open, io::Error::last_os_error()));
        }
        self.handle = Some(unsafe { OwnedHandle::from_raw_handle(handle.cast()) });

        Ok(())
    }

    fn close(&mut self) -> io::Result<()> {
        let Some(handle) = self.handle.take() else {
            return Ok(());
        };

        let closed = unsafe { winapi::um::handleapi::CloseHandle(handle.into_raw_handle().cast()) };
        if closed != winapi::shared::minwindef::TRUE {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    pub fn lock(&mut self) -> Result<FileLockGuard<'_>> {
        self.open()?;
        let handle: winapi::um::winnt::HANDLE =
            self.handle.as_ref().unwrap().as_raw_handle().cast();

        #[allow(dangling_pointers_from_temporaries)]
        unsafe {
            let mut overlapped: winapi::um::minwinbase::OVERLAPPED = winapi::_core::mem::zeroed();
            let locked = winapi::um::fileapi::LockFileEx(
                handle,
                winapi::um::minwinbase::LOCKFILE_EXCLUSIVE_LOCK,
                0,
                !0,
                !0,
                &mut overlapped as *mut winapi::um::minwinbase::OVERLAPPED,
            );

            if locked != winapi::shared::minwindef::TRUE {
                let lock_error = io::Error::last_os_error();
                let _ = self.close();
                return Err(Error::new(ErrorOperation::Lock, lock_error));
            }
        }
        Ok(FileLockGuard::new(self))
    }

    /// Attempts to acquire the lock without blocking.
    ///
    /// Returns `Ok(None)` when another process or thread currently holds the
    /// lock, and `Ok(Some(_))` when the lock was acquired.
    pub fn try_lock(&mut self) -> Result<Option<FileLockGuard<'_>>> {
        self.open()?;
        let handle: winapi::um::winnt::HANDLE =
            self.handle.as_ref().unwrap().as_raw_handle().cast();

        unsafe {
            let mut overlapped: winapi::um::minwinbase::OVERLAPPED = winapi::_core::mem::zeroed();
            let locked = winapi::um::fileapi::LockFileEx(
                handle,
                winapi::um::minwinbase::LOCKFILE_EXCLUSIVE_LOCK
                    | winapi::um::minwinbase::LOCKFILE_FAIL_IMMEDIATELY,
                0,
                !0,
                !0,
                &mut overlapped,
            );

            if locked != winapi::shared::minwindef::TRUE {
                let lock_error = io::Error::last_os_error();
                let _ = self.close();
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
        let Some(handle) = self
            .handle
            .as_ref()
            .map(AsRawHandle::as_raw_handle)
            .map(|handle| handle.cast())
        else {
            return Ok(());
        };

        let unlock_error = unsafe {
            let mut overlapped: winapi::um::minwinbase::OVERLAPPED = winapi::_core::mem::zeroed();
            let unlocked = winapi::um::fileapi::UnlockFileEx(
                handle,
                0,
                !0,
                !0,
                &mut overlapped as *mut winapi::um::minwinbase::OVERLAPPED,
            );

            (unlocked != winapi::shared::minwindef::TRUE).then(io::Error::last_os_error)
        };

        let close_result = self.close();

        if let Some(error) = unlock_error {
            return Err(Error::new(ErrorOperation::Unlock, error));
        }
        close_result.map_err(|error| Error::new(ErrorOperation::Close, error))
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = self.close();

        // The lock file is intentionally left on disk; deleting it on release
        // would open a window for concurrent processes to bypass the lock.
    }
}
