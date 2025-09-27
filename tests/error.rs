use filelock::FileLock;

#[test]
fn test_invalid_filename() {
    let mut lock = FileLock::new("/invalid/path/test.lock");
    let result = lock.lock();
    assert!(result.is_err());
}

#[test]
fn test_double_lock_same_instance() {
    let mut lock = FileLock::new("test_double.lock");
    let guard1 = lock.lock().unwrap();

    drop(guard1);

    let _guard2 = lock.lock().unwrap();
}

#[test]
fn test_file_permissions() {
    if cfg!(unix) {
        use std::fs::File;
        use std::os::unix::fs::PermissionsExt;

        let filename = "test_permissions.lock";
        File::create(filename).unwrap();
        std::fs::set_permissions(filename, std::fs::Permissions::from_mode(0o000)).unwrap();

        let mut lock = FileLock::new(filename);
        let result = lock.lock();
        assert!(result.is_err());

        std::fs::set_permissions(filename, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::remove_file(filename).unwrap();
    }
}