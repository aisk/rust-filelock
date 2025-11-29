extern crate winapi;

use crate::FileLockGuard;
use std::ffi::CString;

pub struct FileLock {
    handle: winapi::um::winnt::HANDLE,
    create_error: Option<errno::Errno>,
}

impl FileLock {
    pub fn new(filename: &str) -> FileLock {
        #[allow(dangling_pointers_from_temporaries)]
        let handle = unsafe {
            winapi::um::fileapi::CreateFileA(
                CString::new(filename).unwrap().as_ptr(),
                winapi::um::winnt::GENERIC_READ | winapi::um::winnt::GENERIC_WRITE,
                winapi::um::winnt::FILE_SHARE_READ | winapi::um::winnt::FILE_SHARE_WRITE,
                0 as *mut winapi::um::minwinbase::SECURITY_ATTRIBUTES,
                winapi::um::fileapi::OPEN_ALWAYS,
                winapi::um::winnt::FILE_ATTRIBUTE_NORMAL,
                winapi::shared::ntdef::NULL,
            )
        };

        // Save error if CreateFileA failed, to avoid errno race conditions
        let create_error = if handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            Some(errno::errno())
        } else {
            None
        };

        return FileLock {
            handle: handle,
            create_error: create_error,
        };
    }

    pub fn lock(&mut self) -> Result<FileLockGuard<'_>, errno::Errno> {
        // Check if file handle is valid from new()
        if self.handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            return Err(self.create_error.unwrap_or(errno::errno()));
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
                return Err(errno::errno());
            }
        }
        return Ok(FileLockGuard::new(self));
    }

    pub(crate) fn unlock(&mut self) -> Result<(), errno::Errno> {
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
                return Err(errno::errno());
            }

            // Don't close handle here - it will be closed when FileLock is dropped
        }

        return Ok(());
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        if self.handle != winapi::shared::ntdef::NULL && self.handle != winapi::um::handleapi::INVALID_HANDLE_VALUE {
            unsafe {
                winapi::um::handleapi::CloseHandle(self.handle);
            }
        }
    }
}
