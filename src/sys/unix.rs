// Derived from Rust 1.97.1's library/std/src/sys/fs/unix.rs:
// https://github.com/rust-lang/rust/blob/1.97.1/library/std/src/sys/fs/unix.rs
// The Rust standard library is licensed under Apache-2.0 OR MIT.

use super::TryLockError;
use std::fs::File;
use std::io;
use std::os::unix::io::AsRawFd;

#[cfg(target_os = "solaris")]
use std::mem;

#[cfg(any(
    target_os = "freebsd",
    target_os = "fuchsia",
    target_os = "hurd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "illumos",
    target_os = "aix",
    target_vendor = "apple",
))]
pub(crate) fn lock(file: &File) -> io::Result<()> {
    cvt(unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) }).map(drop)
}

#[cfg(target_os = "solaris")]
pub(crate) fn lock(file: &File) -> io::Result<()> {
    fcntl_lock(file, libc::F_WRLCK as libc::c_short, libc::F_SETLKW)
}

#[cfg(not(any(
    target_os = "freebsd",
    target_os = "fuchsia",
    target_os = "hurd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "solaris",
    target_os = "illumos",
    target_os = "aix",
    target_vendor = "apple",
)))]
pub(crate) fn lock(_file: &File) -> io::Result<()> {
    unsupported("lock() not supported")
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "fuchsia",
    target_os = "hurd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "illumos",
    target_os = "aix",
    target_vendor = "apple",
))]
pub(crate) fn lock_shared(file: &File) -> io::Result<()> {
    cvt(unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_SH) }).map(drop)
}

#[cfg(target_os = "solaris")]
pub(crate) fn lock_shared(file: &File) -> io::Result<()> {
    fcntl_lock(file, libc::F_RDLCK as libc::c_short, libc::F_SETLKW)
}

#[cfg(not(any(
    target_os = "freebsd",
    target_os = "fuchsia",
    target_os = "hurd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "solaris",
    target_os = "illumos",
    target_os = "aix",
    target_vendor = "apple",
)))]
pub(crate) fn lock_shared(_file: &File) -> io::Result<()> {
    unsupported("lock_shared() not supported")
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "fuchsia",
    target_os = "hurd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "illumos",
    target_os = "aix",
    target_vendor = "apple",
))]
pub(crate) fn try_lock(file: &File) -> Result<(), TryLockError> {
    convert_try_result(cvt(unsafe {
        libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB)
    }))
}

#[cfg(target_os = "solaris")]
pub(crate) fn try_lock(file: &File) -> Result<(), TryLockError> {
    convert_try_result(fcntl_lock(
        file,
        libc::F_WRLCK as libc::c_short,
        libc::F_SETLK,
    ))
}

#[cfg(not(any(
    target_os = "freebsd",
    target_os = "fuchsia",
    target_os = "hurd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "solaris",
    target_os = "illumos",
    target_os = "aix",
    target_vendor = "apple",
)))]
pub(crate) fn try_lock(_file: &File) -> Result<(), TryLockError> {
    Err(TryLockError::Error(
        unsupported("try_lock() not supported").unwrap_err(),
    ))
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "fuchsia",
    target_os = "hurd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "illumos",
    target_os = "aix",
    target_vendor = "apple",
))]
pub(crate) fn try_lock_shared(file: &File) -> Result<(), TryLockError> {
    convert_try_result(cvt(unsafe {
        libc::flock(file.as_raw_fd(), libc::LOCK_SH | libc::LOCK_NB)
    }))
}

#[cfg(target_os = "solaris")]
pub(crate) fn try_lock_shared(file: &File) -> Result<(), TryLockError> {
    convert_try_result(fcntl_lock(
        file,
        libc::F_RDLCK as libc::c_short,
        libc::F_SETLK,
    ))
}

#[cfg(not(any(
    target_os = "freebsd",
    target_os = "fuchsia",
    target_os = "hurd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "solaris",
    target_os = "illumos",
    target_os = "aix",
    target_vendor = "apple",
)))]
pub(crate) fn try_lock_shared(_file: &File) -> Result<(), TryLockError> {
    Err(TryLockError::Error(
        unsupported("try_lock_shared() not supported").unwrap_err(),
    ))
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "fuchsia",
    target_os = "hurd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "illumos",
    target_os = "aix",
    target_vendor = "apple",
))]
pub(crate) fn unlock(file: &File) -> io::Result<()> {
    cvt(unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) }).map(drop)
}

#[cfg(target_os = "solaris")]
pub(crate) fn unlock(file: &File) -> io::Result<()> {
    fcntl_lock(file, libc::F_UNLCK as libc::c_short, libc::F_SETLKW)
}

#[cfg(not(any(
    target_os = "freebsd",
    target_os = "fuchsia",
    target_os = "hurd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "solaris",
    target_os = "illumos",
    target_os = "aix",
    target_vendor = "apple",
)))]
pub(crate) fn unlock(_file: &File) -> io::Result<()> {
    unsupported("unlock() not supported")
}

#[cfg(target_os = "solaris")]
fn fcntl_lock(file: &File, lock_type: libc::c_short, command: libc::c_int) -> io::Result<()> {
    let mut lock: libc::flock = unsafe { mem::zeroed() };
    lock.l_type = lock_type;
    lock.l_whence = libc::SEEK_SET as libc::c_short;
    cvt(unsafe { libc::fcntl(file.as_raw_fd(), command, &lock) }).map(drop)
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "fuchsia",
    target_os = "hurd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "solaris",
    target_os = "illumos",
    target_os = "aix",
    target_vendor = "apple",
))]
fn cvt(result: libc::c_int) -> io::Result<libc::c_int> {
    if result == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result)
    }
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "fuchsia",
    target_os = "hurd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "solaris",
    target_os = "illumos",
    target_os = "aix",
    target_vendor = "apple",
))]
fn convert_try_result(result: io::Result<libc::c_int>) -> Result<(), TryLockError> {
    match result {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Err(TryLockError::WouldBlock),
        Err(error) => Err(TryLockError::Error(error)),
    }
}

#[cfg(not(any(
    target_os = "freebsd",
    target_os = "fuchsia",
    target_os = "hurd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "solaris",
    target_os = "illumos",
    target_os = "aix",
    target_vendor = "apple",
)))]
fn unsupported(message: &'static str) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, message))
}
