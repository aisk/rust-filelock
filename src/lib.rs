#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::FileLock;

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::FileLock;

pub fn new(filename: &str) -> FileLock {
    return FileLock::new(filename);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
        let mut lock = super::new("test.lock");
        lock.lock().unwrap();
        lock.unlock().unwrap();
    }
}
