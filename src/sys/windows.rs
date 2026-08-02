// Derived from Rust 1.97.1's library/std/src/sys/fs/windows.rs:
// https://github.com/rust-lang/rust/blob/1.97.1/library/std/src/sys/fs/windows.rs
// The Rust standard library is licensed under Apache-2.0 OR MIT.

use super::TryLockError;
use std::fs::File;
use std::io;
use std::mem;
use std::os::windows::io::AsRawHandle;
use std::ptr;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_IO_PENDING, ERROR_LOCK_VIOLATION, ERROR_NOT_LOCKED, HANDLE,
};
use windows_sys::Win32::Storage::FileSystem::{
    LockFileEx, UnlockFile, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
};
use windows_sys::Win32::System::Threading::CreateEventW;
use windows_sys::Win32::System::IO::{GetOverlappedResult, OVERLAPPED};

pub(crate) fn lock(file: &File) -> io::Result<()> {
    acquire_lock(file, LOCKFILE_EXCLUSIVE_LOCK)
}

pub(crate) fn lock_shared(file: &File) -> io::Result<()> {
    acquire_lock(file, 0)
}

pub(crate) fn try_lock(file: &File) -> Result<(), TryLockError> {
    try_acquire_lock(file, LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY)
}

pub(crate) fn try_lock_shared(file: &File) -> Result<(), TryLockError> {
    try_acquire_lock(file, LOCKFILE_FAIL_IMMEDIATELY)
}

pub(crate) fn unlock(file: &File) -> io::Result<()> {
    let handle = raw_handle(file);
    cvt(unsafe { UnlockFile(handle, 0, 0, u32::MAX, u32::MAX) })?;

    // LockFileEx permits a shared and an exclusive lock on the same handle.
    // In that case Windows requires two unlock operations for the region.
    match cvt(unsafe { UnlockFile(handle, 0, 0, u32::MAX, u32::MAX) }) {
        Ok(_) => Ok(()),
        Err(error) if error.raw_os_error() == Some(ERROR_NOT_LOCKED as i32) => Ok(()),
        Err(error) => Err(error),
    }
}

fn acquire_lock(file: &File, flags: u32) -> io::Result<()> {
    unsafe {
        let mut overlapped: OVERLAPPED = mem::zeroed();
        let event = CreateEventW(ptr::null(), 0, 0, ptr::null());
        if event == 0 {
            return Err(io::Error::last_os_error());
        }
        overlapped.hEvent = event;

        let result = cvt(LockFileEx(
            raw_handle(file),
            flags,
            0,
            u32::MAX,
            u32::MAX,
            &mut overlapped,
        ));
        let result = match result {
            Ok(_) => Ok(()),
            Err(error) if error.raw_os_error() == Some(ERROR_IO_PENDING as i32) => {
                let mut bytes_transferred = 0;
                cvt(GetOverlappedResult(
                    raw_handle(file),
                    &mut overlapped,
                    &mut bytes_transferred,
                    1,
                ))
                .map(drop)
            }
            Err(error) => Err(error),
        };

        CloseHandle(event);
        result
    }
}

fn try_acquire_lock(file: &File, flags: u32) -> Result<(), TryLockError> {
    let mut overlapped: OVERLAPPED = unsafe { mem::zeroed() };
    match cvt(unsafe {
        LockFileEx(
            raw_handle(file),
            flags,
            0,
            u32::MAX,
            u32::MAX,
            &mut overlapped,
        )
    }) {
        Ok(_) => Ok(()),
        Err(error) if error.raw_os_error() == Some(ERROR_LOCK_VIOLATION as i32) => {
            Err(TryLockError::WouldBlock)
        }
        Err(error) => Err(TryLockError::Error(error)),
    }
}

fn raw_handle(file: &File) -> HANDLE {
    file.as_raw_handle() as HANDLE
}

fn cvt(result: i32) -> io::Result<i32> {
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::{lock, lock_shared, try_lock, unlock};
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OVERLAPPED;

    fn path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("filelock-windows-{name}-{}", std::process::id()))
    }

    #[test]
    fn blocking_lock_waits_for_overlapped_handles() {
        let path = path("overlapped");
        let holder = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .unwrap();
        let waiter = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(FILE_FLAG_OVERLAPPED)
            .open(path)
            .unwrap();

        lock(&holder).unwrap();
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let thread = thread::spawn(move || {
            lock(&waiter).unwrap();
            acquired_tx.send(()).unwrap();
        });

        assert_eq!(
            acquired_rx.recv_timeout(Duration::from_millis(100)),
            Err(mpsc::RecvTimeoutError::Timeout)
        );
        unlock(&holder).unwrap();
        acquired_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        thread.join().unwrap();
    }

    #[test]
    fn one_unlock_releases_stacked_shared_and_exclusive_locks() {
        let path = path("stacked");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .unwrap();
        let contender = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap();

        lock(&file).unwrap();
        lock_shared(&file).unwrap();
        unlock(&file).unwrap();
        try_lock(&contender).expect("both locks should have been released");
        unlock(&contender).unwrap();
    }
}
