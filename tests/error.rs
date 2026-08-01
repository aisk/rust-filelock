use filelock::{ErrorOperation, FileLock};

#[test]
fn test_open_error_recovery() {
    let directory = std::env::temp_dir().join(format!("filelock-recovery-{}", std::process::id()));
    let filename = directory.join("test.lock");

    let mut lock = FileLock::new(&filename);
    let error = match lock.lock() {
        Err(error) => error,
        Ok(_) => panic!("locking in a missing directory unexpectedly succeeded"),
    };
    assert_eq!(error.operation(), ErrorOperation::Open);

    std::fs::create_dir(&directory).unwrap();
    lock.lock().unwrap().unlock().unwrap();

    std::fs::remove_file(filename).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn test_invalid_path_locking() {
    let mut lock = FileLock::new("/invalid/path/that/does/not/exist/test.lock");
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

    let prefix =
        std::env::temp_dir().join(format!("filelock-nul-prefix-{}.lock", std::process::id()));
    let _ = std::fs::remove_file(&prefix);

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

    let filename = "test_permissions.lock";

    // Create a file with no permissions
    File::create(filename).unwrap();
    std::fs::set_permissions(filename, std::fs::Permissions::from_mode(0o000)).unwrap();

    let mut lock = FileLock::new(filename);
    let result = lock.lock();
    assert!(
        result.is_err(),
        "Locking on file with no permissions should fail"
    );

    // Cleanup: restore permissions and remove file
    std::fs::set_permissions(filename, std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::remove_file(filename).unwrap();
}

#[test]
fn test_error_recovery() {
    let filename = "test_error_recovery.lock";
    let mut lock = FileLock::new(filename);

    // First successful lock
    let guard1 = lock.lock().unwrap();
    drop(guard1);

    // Second lock should work fine after first is released
    let _guard2 = lock.lock().unwrap();
}
