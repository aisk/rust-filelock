# filelock

[![Rust](https://github.com/aisk/rust-filelock/actions/workflows/ci.yml/badge.svg)](https://github.com/aisk/rust-filelock/actions/workflows/ci.yml)

Simple filelock library for rust, using `flock` on Unix-like systems and `LockFileEx` on Windows under the hood.

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
    let mut lock = filelock::new("myfile.lock");
    let _guard = lock.lock()?;

    // Perform critical operations

    // Lock is automatically released when _guard goes out of scope
    Ok(())
}
```

For manual control:

```rust
use filelock;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut lock = filelock::new("myfile.lock");
    let guard = lock.lock()?;

    // Perform critical operations

    // Manually unlock with error handling
    guard.unlock()?;

    Ok(())
}
```

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

`lock_async()` uses non-blocking lock attempts and asynchronous, bounded
backoff, so lock contention does not park a Tokio runtime worker. It can be
used with `tokio::time::timeout` or `tokio::select!`, and cancelling the wait
does not leave a blocking task running in the background. Each attempt still
uses an ordinary synchronous filesystem call to open the lock file; that call
may itself block on a slow or remote filesystem. Do not call the synchronous
`lock()` from a Tokio runtime worker when the lock may be contended.

Errors are reported as `filelock::Error`. The error identifies which operation failed and exposes both a portable `std::io::ErrorKind` and, when available, the platform-specific error code:

```rust
use filelock::{ErrorOperation, FileLock};

let mut lock = FileLock::new("missing/directory/myfile.lock");
if let Err(error) = lock.lock() {
    assert_eq!(error.operation(), ErrorOperation::Open);
    eprintln!("{} ({:?})", error, error.kind());
}
```

## Platform behavior

- Every participant must use this locking protocol and resolve the same stable lock-file path. Do not delete, rename, or replace the lock file while participants may be running; doing so can let processes lock different underlying files.
- Unix uses advisory `flock` locks. Uncooperative processes can still access the lock file, and a process must not `fork` while holding a guard and then let both parent and child continue through the protected critical section.
- Unix lock files are opened read-write and created with mode `0644` before applying the process umask. Cross-user locking therefore requires permissions to be arranged explicitly.
- Windows uses an exclusive byte-range lock. It prevents ordinary reads and writes to the locked range by other processes, but does not prevent access through memory-mapped views.
- Network filesystem behavior depends on the operating system, filesystem, mount options, and server. Validate the required semantics before using a lock file on NFS, SMB, or another remote filesystem.

## Documentation

See https://docs.rs/filelock/latest/filelock/.

## License

Filelock is distributed by a [MIT license](https://github.com/aisk/rust-filelock/tree/master/LICENSE).
