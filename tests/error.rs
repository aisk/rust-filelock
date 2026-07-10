use filelock::FileLock;

#[test]
fn test_invalid_path_locking() {
    let mut lock = FileLock::new("/invalid/path/that/does/not/exist/test.lock");
    let result = lock.lock();
    assert!(result.is_err(), "Locking on invalid path should fail");
}

#[cfg(unix)]
#[test]
fn test_path_with_interior_nul_returns_error() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let mut lock = FileLock::new(OsStr::from_bytes(b"invalid\0path.lock"));
    let result = lock.lock();

    assert_eq!(result.err(), Some(errno::Errno(libc::EINVAL)));
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
