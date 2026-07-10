# filelock

[![Rust](https://github.com/aisk/filelock/actions/workflows/ci.yml/badge.svg)](https://github.com/aisk/filelock/actions/workflows/ci.yml)

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

Errors are reported as `filelock::Error`. The error identifies which operation failed and exposes both a portable `std::io::ErrorKind` and, when available, the platform-specific error code:

```rust
use filelock::{ErrorOperation, FileLock};

let mut lock = FileLock::new("missing/directory/myfile.lock");
if let Err(error) = lock.lock() {
    assert_eq!(error.operation(), ErrorOperation::Open);
    eprintln!("{} ({:?})", error, error.kind());
}
```

## Documentation

See https://docs.rs/filelock/latest/filelock/.

## License

Filelock is distributed by a [MIT license](https://github.com/aisk/filelock/tree/master/LICENSE).
