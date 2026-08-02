//! Internal compatibility layer for the standard library's file-locking API.

use std::io;

#[derive(Debug)]
pub(crate) enum TryLockError {
    Error(io::Error),
    #[allow(dead_code)] // No contention variant is produced on unsupported targets.
    WouldBlock,
}

#[cfg(filelock_std_lock)]
#[allow(clippy::incompatible_msrv)]
mod imp {
    use super::{io, TryLockError};
    use std::fs::File;

    pub(crate) fn lock(file: &File) -> io::Result<()> {
        file.lock()
    }

    pub(crate) fn lock_shared(file: &File) -> io::Result<()> {
        file.lock_shared()
    }

    pub(crate) fn try_lock(file: &File) -> Result<(), TryLockError> {
        file.try_lock().map_err(convert_try_lock_error)
    }

    pub(crate) fn try_lock_shared(file: &File) -> Result<(), TryLockError> {
        file.try_lock_shared().map_err(convert_try_lock_error)
    }

    pub(crate) fn unlock(file: &File) -> io::Result<()> {
        file.unlock()
    }

    fn convert_try_lock_error(error: std::fs::TryLockError) -> TryLockError {
        match error {
            std::fs::TryLockError::Error(error) => TryLockError::Error(error),
            std::fs::TryLockError::WouldBlock => TryLockError::WouldBlock,
        }
    }
}

#[cfg(all(not(filelock_std_lock), unix))]
mod unix;
#[cfg(all(not(filelock_std_lock), unix))]
use unix as imp;

#[cfg(all(not(filelock_std_lock), windows))]
mod windows;
#[cfg(all(not(filelock_std_lock), windows))]
use windows as imp;

#[cfg(any(all(not(filelock_std_lock), not(any(unix, windows))), test))]
mod unsupported;
#[cfg(all(not(filelock_std_lock), not(any(unix, windows))))]
use unsupported as imp;

pub(crate) use imp::{lock, lock_shared, try_lock, try_lock_shared, unlock};

#[cfg(all(test, filelock_has_std_lock, not(filelock_std_lock)))]
#[allow(clippy::incompatible_msrv)]
mod interoperability_tests {
    use super::{lock, try_lock, unlock, TryLockError};
    use std::fs::{File, OpenOptions, TryLockError as StdTryLockError};

    fn files(name: &str) -> (File, File) {
        let path = std::env::temp_dir().join(format!(
            "filelock-interoperability-{name}-{}",
            std::process::id()
        ));
        let open = || {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&path)
                .unwrap()
        };
        (open(), open())
    }

    #[test]
    fn backport_lock_blocks_standard_library_lock() {
        let (backport_file, std_file) = files("backport-to-std");
        lock(&backport_file).unwrap();
        assert!(matches!(
            std_file.try_lock(),
            Err(StdTryLockError::WouldBlock)
        ));
        unlock(&backport_file).unwrap();
    }

    #[test]
    fn standard_library_lock_blocks_backport_lock() {
        let (std_file, backport_file) = files("std-to-backport");
        std_file.lock().unwrap();
        assert!(matches!(
            try_lock(&backport_file),
            Err(TryLockError::WouldBlock)
        ));
        std_file.unlock().unwrap();
    }
}
