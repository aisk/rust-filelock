extern crate libc;

use std::fs::File;
use std::os::unix::io::AsRawFd;


pub struct FileLock {
    filename: String
}

impl FileLock {
    pub fn new(filename: &str) -> FileLock {
        return FileLock{filename: filename.to_string()}
    }

    pub fn lock(self) {
        let f = File::open(self.filename).unwrap();
        unsafe {
            let r = libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB);
            println!("r: {}", r);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
        super::FileLock::new("/tmp/test.lock").lock();
    }
}
