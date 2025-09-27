# filelock

[![Rust](https://github.com/aisk/filelock/actions/workflows/ci.yml/badge.svg)](https://github.com/aisk/filelock/actions/workflows/ci.yml)

Simple filelock library for rust, using `flock` on Unix-like systems and `LockFileEx` on Windows under the hood.

## Installation

```sh
$ cargo add filelock
```

## Usage

```rust
use filelock::FileLock;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut lock = FileLock::new("myfile.lock");
    let _guard = lock.lock()?;

    // Perform critical operations

    // Lock is automatically released when _guard goes out of scope
    Ok(())
}
```

For manual control:

```rust
use filelock::FileLock;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut lock = FileLock::new("myfile.lock");
    let guard = lock.lock()?;

    // Perform critical operations

    // Manually unlock with error handling
    guard.unlock()?;

    Ok(())
}
```

## Documentation

See https://docs.rs/filelock/latest/filelock/.

## License

Filelock is distributed by a [MIT license](https://github.com/aisk/filelock/tree/master/LICENSE).
