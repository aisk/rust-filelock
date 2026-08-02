# filelock

[![Rust](https://github.com/aisk/rust-filelock/actions/workflows/ci.yml/badge.svg)](https://github.com/aisk/rust-filelock/actions/workflows/ci.yml)

Simple filelock library for rust, built on the standard library's file locking support (`std::fs::File::lock` and friends, stable since Rust 1.89). The standard library uses `flock` (or `fcntl` where `flock` is unavailable) on Unix-like systems and `LockFileEx` on Windows.

![](https://repository-images.githubusercontent.com/403675076/cd5f3635-33cf-4905-8315-1e7aee048c0d)

*Image by Homutan, source: https://www.pixiv.net/artworks/128080460*

## Installation

```sh
$ cargo add filelock
```

## Usage

```rust
use filelock;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut lock = filelock::new("myfile.lock")?;
    let _guard = lock.lock()?;

    // Perform critical operations

    // Lock is automatically released when _guard goes out of scope
    Ok(())
}
```

`filelock::new` opens (and creates if missing) the lock file, and the file
stays open for the lifetime of the `FileLock`, so repeated lock/unlock cycles
reuse the same file descriptor or handle.

To attempt locking without waiting, or to take a shared (read) lock that
multiple holders can share while excluding exclusive locks:

```rust
let mut lock = filelock::new("myfile.lock")?;

if let Some(_guard) = lock.try_lock()? {
    // The exclusive lock was acquired.
}

let _guard = lock.lock_shared()?;
// try_lock_shared() is also available.
```

To wait with an upper bound, use `lock_timeout` / `lock_shared_timeout`:

```rust
use std::time::Duration;

let mut lock = filelock::new("myfile.lock")?;
if let Some(_guard) = lock.lock_timeout(Duration::from_secs(5))? {
    // The lock was acquired within five seconds.
} else {
    // The lock is still held elsewhere.
}
```

The guards returned by `lock()` and friends borrow the `FileLock`. When a
guard needs to outlive the current scope — stored in a struct or moved into a
spawned task — use the owned variants (`lock_owned`, `lock_shared_owned`, and
with the `tokio` feature `lock_owned_async` / `lock_shared_owned_async`),
which take ownership of the `FileLock` and return it on unlock:

```rust
let guard = filelock::new("myfile.lock")?.lock_owned()?;
std::thread::spawn(move || {
    let _guard = guard;
    // Critical section runs on another thread.
});
```

For manual control:

```rust
use filelock;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut lock = filelock::new("myfile.lock")?;
    let guard = lock.lock()?;

    // Perform critical operations

    // Manually unlock with error handling
    guard.unlock()?;

    Ok(())
}
```

An already-open `std::fs::File` can also be locked via
`FileLock::from_file(file)`, and the underlying file is accessible through
`FileLockGuard::file()` while the lock is held (for example to store the
holder's process id in the lock file).

### Tokio

Enable the optional `tokio` feature to wait for a contended lock without
blocking a Tokio runtime worker:

```sh
cargo add filelock --features tokio
cargo add tokio --features macros,rt-multi-thread,time
```

The second command is unnecessary when the application already has a Tokio
dependency with a runtime, macros, and time enabled.

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut lock = filelock::new("myfile.lock");
    let _guard = lock.lock_async().await?;

    // Perform async critical operations

    Ok(())
}
```

`lock_async()` (and `lock_shared_async()`) use non-blocking lock attempts and
asynchronous, bounded backoff, so lock contention does not park a Tokio
runtime worker. They can be used with `tokio::time::timeout` or
`tokio::select!`, and cancelling the wait does not leave a blocking task
running in the background. Do not call the synchronous `lock()` from a Tokio
runtime worker when the lock may be contended.

Errors are reported as `filelock::Error`. The error identifies which operation failed and exposes both a portable `std::io::ErrorKind` and, when available, the platform-specific error code:

```rust
use filelock::{ErrorOperation, FileLock};

if let Err(error) = FileLock::new("missing/directory/myfile.lock") {
    assert_eq!(error.operation(), ErrorOperation::Open);
    eprintln!("{} ({:?})", error, error.kind());
}
```

## Platform behavior

- Every participant must use this locking protocol and resolve the same stable lock-file path. Do not delete, rename, or replace the lock file while participants may be running; doing so can let processes lock different underlying files.
- Locks are advisory and associated with the opened file, not its path. Uncooperative processes can still access the lock file, and a process must not `fork` while holding a guard and then let both parent and child continue through the protected critical section.
- Lock files are opened read-write and, on Unix, created with mode `0644` before applying the process umask. Cross-user locking therefore requires permissions to be arranged explicitly.
- Interactions between file locks and ordinary reads and writes by non-lockholders are platform specific; see the `std::fs::File::lock` documentation for details.
- Network filesystem behavior depends on the operating system, filesystem, mount options, and server. Validate the required semantics before using a lock file on NFS, SMB, or another remote filesystem.

## Minimum supported Rust version

Rust 1.89, which stabilized `std`'s file locking APIs.

## Documentation

See https://docs.rs/filelock/latest/filelock/.

## License

Filelock is distributed by a [MIT license](https://github.com/aisk/rust-filelock/tree/master/LICENSE).
