extern crate winapi;

use std::ffi::CString;

pub struct FileLock {
    filename: String,
    handle: winapi::um::winnt::HANDLE,
}

impl FileLock {
    pub fn new(filename: &str) -> FileLock {
        return FileLock {
            filename: filename.to_string(),
            handle: winapi::shared::ntdef::NULL,
        };
    }

    pub fn lock(&mut self) -> Result<(), errno::Errno> {
        #[allow(temporary_cstring_as_ptr)]
        unsafe {
            let handle = winapi::um::fileapi::CreateFileA(
                CString::new(self.filename.as_str()).unwrap().as_ptr(),
                winapi::um::winnt::GENERIC_READ,
                winapi::um::winnt::FILE_SHARE_READ | winapi::um::winnt::FILE_SHARE_WRITE,
                0 as *mut winapi::um::minwinbase::SECURITY_ATTRIBUTES,
                winapi::um::fileapi::OPEN_ALWAYS,
                winapi::um::winnt::FILE_ATTRIBUTE_NORMAL,
                winapi::shared::ntdef::NULL,
            );
            if handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
                return Err(errno::errno());
            }
            self.handle = handle;

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
        return Ok(());
    }

    pub fn unlock(&mut self) -> Result<(), errno::Errno> {
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
        }

        return Ok(());
    }
}
