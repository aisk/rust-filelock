# filelock

[![Rust](https://github.com/aisk/rust-filelock/actions/workflows/ci.yml/badge.svg)](https://github.com/aisk/rust-filelock/actions/workflows/ci.yml)

A simple file locking library for Rust. On Rust 1.89 and newer, locking uses
the standard library's `std::fs::File::lock` API. Rust 1.71 through 1.88 use a
backport of that implementation from Rust 1.97.1, preserving the same
platform behavior and interoperability.

![](https://repository-images.githubusercontent.com/403675076/cd5f3635-33cf-4905-8315-1e7aee048c0d)

*Image by Homutan, source: https://www.pixiv.net/artworks/128080460*

## Installation

```sh
$ cargo add filelock
```

## Usage

Acquire an exclusive lock, and let the guard release it:

```rust,no_run
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
reuse the same file descriptor or handle. All operations return
`std::io::Result`.

A guard can also be released manually, which reports errors instead of
ignoring them:

```rust,no_run
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut lock = filelock::new("myfile.lock")?;
    let guard = lock.lock()?;

    // Perform critical operations

    guard.unlock()?;
    Ok(())
}
```

### Non-blocking, shared, and bounded waits

`try_lock` attempts to lock without waiting, and `lock_timeout` waits with an
upper bound. Shared (read) locks allow multiple holders at once, while still
excluding exclusive locks:

```rust,no_run
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut lock = filelock::new("myfile.lock")?;

    if let Some(_guard) = lock.try_lock()? {
        // The exclusive lock was acquired without waiting.
    }

    if let Some(_guard) = lock.lock_timeout(Duration::from_secs(5))? {
        // The lock was acquired within five seconds.
    }

    let _guard = lock.lock_shared()?;
    // try_lock_shared() and lock_shared_timeout() are also available.

    Ok(())
}
```

### Owned guards

The guards returned by `lock()` and friends borrow the `FileLock`. When a
guard needs to outlive the current scope, for example stored in a struct or
moved into a spawned task, use the owned variants (`lock_owned`,
`lock_shared_owned`, and with the `tokio` feature `lock_owned_async` /
`lock_shared_owned_async`), which take ownership of the `FileLock` and return
it on unlock:

```rust,no_run
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let guard = filelock::new("myfile.lock")?.lock_owned()?;
    std::thread::spawn(move || {
        let _guard = guard;
        // Critical section runs on another thread.
    });
    Ok(())
}
```

### Using an existing file

An already-open `std::fs::File` can be locked via `FileLock::from_file`,
which allows customizing how the lock file is opened. While the lock is held,
the underlying file is accessible through the guard's `file()` method, for
example to store the holder's process id in the lock file:

```rust,no_run
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("myfile.lock")?;
    let mut lock = filelock::FileLock::from_file(file);
    let _guard = lock.lock()?;
    Ok(())
}
```

On Windows, `from_file` also supports handles opened for overlapped I/O.

### Tokio

Enable the optional `tokio` feature to wait for a contended lock without
blocking a Tokio runtime worker:

```sh
cargo add filelock --features tokio
```

```rust,ignore
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut lock = filelock::new("myfile.lock")?;
    let _guard = lock.lock_async().await?;

    // Perform async critical operations

    Ok(())
}
```

`lock_async()` and `lock_shared_async()` use non-blocking lock attempts and
asynchronous, bounded backoff, so lock contention does not park a Tokio
runtime worker. They can be used with `tokio::time::timeout` or
`tokio::select!`, and cancelling the wait does not leave a blocking task
running in the background. Waiting is polling-based, with the same caveats as
`lock_timeout`: acquisition can lag the lock's release by up to 50ms, and
waiters are not queued in FIFO order under sustained contention. Do not call
the synchronous `lock()` from a Tokio runtime worker when the lock may be
contended.

## Platform behavior

On Linux, the BSDs, Apple platforms, Illumos, AIX, and the other Unix targets
supported by the standard-library implementation, locks use `flock`. Solaris
uses `fcntl` record locks covering the entire file. Windows uses `LockFileEx`.
Other targets return `std::io::ErrorKind::Unsupported` from lock operations.

- Every participant must use this locking protocol and resolve the same stable lock-file path. Do not delete, rename, or replace the lock file while participants may be running; doing so can let processes lock different underlying files.
- Locks are advisory and associated with the opened file, not its path. Uncooperative processes can still access the lock file, and a process must not `fork` while holding a guard and then let both parent and child continue through the protected critical section.
- Lock files are opened read-write and, on Unix, created with mode `0644` before applying the process umask. Cross-user locking therefore requires permissions to be arranged explicitly.
- Interactions between file locks and ordinary reads and writes by non-lockholders are platform specific; see the `std::fs::File::lock` documentation for details.
- Network filesystem behavior depends on the operating system, filesystem, mount options, and server. Validate the required semantics before using a lock file on NFS, SMB, or another remote filesystem.

## Minimum supported Rust version

Rust 1.71, including when the optional `tokio` feature is enabled. Builds with
Rust 1.89 and newer use the standard library's native file-lock API; Rust
1.71–1.88 automatically use the bundled compatibility implementation. This
backend selection is internal and does not change the public API.

The compatibility implementation is derived from the Rust 1.97.1 standard
library's Unix and Windows file-locking implementations, which are licensed
under Apache-2.0 OR MIT. Source and license attribution is retained in the
corresponding implementation files.

## Documentation

See https://docs.rs/filelock/latest/filelock/.

## License

Filelock is distributed by a [MIT license](https://github.com/aisk/rust-filelock/tree/master/LICENSE).
