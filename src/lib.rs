extern crate errno;
extern crate libc;

use std::ffi::CString;

pub struct FileLock {
    filename: String,
    fd: libc::c_int,
}

impl FileLock {
    pub fn new(filename: &str) -> FileLock {
        return FileLock {
            filename: filename.to_string(),
            fd: 0,
        };
    }

    pub fn lock(&mut self) -> Result<(), errno::Errno> {
        unsafe {
            #[allow(temporary_cstring_as_ptr)]
            let fd = libc::open(
                CString::new(self.filename.as_str()).unwrap().as_ptr(),
                libc::O_RDWR | libc::O_CREAT,
                0o644,
            );
            if fd < 0 {
                return Err(errno::errno());
            }
            self.fd = fd;

            if libc::flock(fd, libc::LOCK_EX) != 0 {
                return Err(errno::errno());
            }
            return Ok(());
        }
    }

    pub fn unlock(&mut self) -> Result<(), errno::Errno> {
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

        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
        let mut lock = super::FileLock::new("/tmp/test.lock");
        lock.lock().unwrap();
        lock.unlock().unwrap();
    }
}
