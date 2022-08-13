pub struct FileLock {
    filename: String,
}

impl FileLock {
    pub fn new(filename: &str) -> FileLock {
        return FileLock {
            filename: filename.to_string(),
        };
    }

    pub fn lock(&mut self) -> Result<(), errno::Errno> {
        return Ok(());
    }

    pub fn unlock(&mut self) -> Result<(), errno::Errno> {
        return Ok(());
    }
}
