use super::TryLockError;
use std::fs::File;
use std::io;

pub(crate) fn lock(_file: &File) -> io::Result<()> {
    unsupported("lock() not supported")
}

pub(crate) fn lock_shared(_file: &File) -> io::Result<()> {
    unsupported("lock_shared() not supported")
}

pub(crate) fn try_lock(_file: &File) -> Result<(), TryLockError> {
    Err(TryLockError::Error(
        unsupported("try_lock() not supported").unwrap_err(),
    ))
}

pub(crate) fn try_lock_shared(_file: &File) -> Result<(), TryLockError> {
    Err(TryLockError::Error(
        unsupported("try_lock_shared() not supported").unwrap_err(),
    ))
}

pub(crate) fn unlock(_file: &File) -> io::Result<()> {
    unsupported("unlock() not supported")
}

fn unsupported(message: &'static str) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, message))
}

#[cfg(test)]
mod tests {
    use super::{lock, lock_shared, try_lock, try_lock_shared, unlock};
    use crate::sys::TryLockError;
    use std::fs::File;
    use std::io::ErrorKind;

    #[test]
    fn every_operation_reports_unsupported() {
        let path =
            std::env::temp_dir().join(format!("filelock-unsupported-test-{}", std::process::id()));
        let file = File::create(path).unwrap();

        assert_eq!(lock(&file).unwrap_err().kind(), ErrorKind::Unsupported);
        assert_eq!(
            lock_shared(&file).unwrap_err().kind(),
            ErrorKind::Unsupported
        );
        assert_try_error(try_lock(&file));
        assert_try_error(try_lock_shared(&file));
        assert_eq!(unlock(&file).unwrap_err().kind(), ErrorKind::Unsupported);
    }

    fn assert_try_error(result: Result<(), TryLockError>) {
        match result {
            Err(TryLockError::Error(error)) => {
                assert_eq!(error.kind(), ErrorKind::Unsupported);
            }
            _ => panic!("operation did not return an unsupported I/O error"),
        }
    }
}
