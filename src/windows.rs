extern crate winapi;

use crate::FileLockGuard;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

pub struct FileLock {
    handle: winapi::um::winnt::HANDLE,
    create_error: Option<errno::Errno>,
}

impl FileLock {
    pub fn new<P: AsRef<Path>>(filename: P) -> FileLock {
        let path = filename.as_ref();

        // Convert path to null-terminated UTF-16 wide string
        let wide_path: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

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

        // Save error if CreateFileW failed, to avoid errno race conditions
        let create_error = if handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            Some(errno::errno())
        } else {
            None
        };

        FileLock {
            handle,
            create_error,
        }
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
