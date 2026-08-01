mod common;

use common::TestDir;
use filelock::{ErrorOperation, FileLock};

#[test]
fn test_open_error_recovery() {
    let test_dir = TestDir::new("open-recovery");
    let directory = test_dir.path("missing");
    let filename = directory.join("test.lock");

    let mut lock = FileLock::new(&filename);
    let error = match lock.lock() {
        Err(error) => error,
        Ok(_) => panic!("locking in a missing directory unexpectedly succeeded"),
    };
    assert_eq!(error.operation(), ErrorOperation::Open);

    std::fs::create_dir(&directory).unwrap();
    lock.lock().unwrap().unlock().unwrap();
}

#[test]
fn test_invalid_path_locking() {
    let test_dir = TestDir::new("invalid-path");
    let parent = test_dir.path("not-a-directory");
    std::fs::write(&parent, b"file").unwrap();
    let mut lock = FileLock::new(parent.join("test.lock"));
    let result = lock.lock();
    assert!(result.is_err(), "Locking on invalid path should fail");
    let error = result.err().unwrap();
    assert_eq!(error.operation(), ErrorOperation::Open);
    assert!(std::error::Error::source(&error).is_some());
    assert!(error.to_string().starts_with("failed to open lock file:"));
}

#[cfg(unix)]
#[test]
fn test_path_with_interior_nul_returns_error() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let mut lock = FileLock::new(OsStr::from_bytes(b"invalid\0path.lock"));
    let result = lock.lock();

    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("path containing NUL unexpectedly succeeded"),
    };
    assert_eq!(error.operation(), ErrorOperation::Open);
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(error.raw_os_error(), None);
}

#[cfg(windows)]
#[test]
fn test_path_with_interior_nul_returns_error() {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    let test_dir = TestDir::new("nul-prefix");
    let prefix = test_dir.path("prefix.lock");

    let mut path: Vec<u16> = prefix.as_os_str().encode_wide().collect();
    path.push(0);
    path.extend("ignored".encode_utf16());

    let mut lock = FileLock::new(std::ffi::OsString::from_wide(&path));
    let error = match lock.lock() {
        Err(error) => error,
        Ok(_) => panic!("path containing NUL unexpectedly succeeded"),
    };

    assert_eq!(error.operation(), ErrorOperation::Open);
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(
        !prefix.exists(),
        "the truncated path prefix was unexpectedly created"
    );
}

#[cfg(unix)]
#[test]
fn test_file_permission_denied() {
    use std::fs::File;
    use std::os::unix::fs::PermissionsExt;

    let test_dir = TestDir::new("permissions");
    let filename = test_dir.path("test.lock");

    // Create a file with no permissions
    File::create(&filename).unwrap();
    std::fs::set_permissions(&filename, std::fs::Permissions::from_mode(0o000)).unwrap();

    let mut lock = FileLock::new(&filename);
    let denied = match lock.lock() {
        Err(error) => Some(error.operation()),
        Ok(guard) => {
            drop(guard);
            None
        }
    };

    std::fs::set_permissions(&filename, std::fs::Permissions::from_mode(0o644)).unwrap();
    if let Some(operation) = denied {
        assert_eq!(operation, ErrorOperation::Open);
    }
}

#[test]
fn test_error_recovery() {
    let test_dir = TestDir::new("error-recovery");
    let mut lock = FileLock::new(test_dir.path("test.lock"));

    // First successful lock
    let guard1 = lock.lock().unwrap();
    drop(guard1);

    // Second lock should work fine after first is released
    let _guard2 = lock.lock().unwrap();
}
