use filelock::FileLock;

#[test]
fn test_basic_lock_unlock() {
    let mut lock = FileLock::new("test_basic.lock");
    let guard = lock.lock().unwrap();
    drop(guard);
    let _guard2 = lock.lock().unwrap();
}

#[test]
fn test_scope_based_locking() {
    let mut lock = FileLock::new("test_scope.lock");

    {
        let _guard = lock.lock().unwrap();
    }

    {
        let _guard = lock.lock().unwrap();
    }
}

#[test]
fn test_multiple_locks_same_file() {
    let mut lock1 = FileLock::new("test_multiple.lock");
    let mut lock2 = FileLock::new("test_multiple.lock");

    let guard1 = lock1.lock().unwrap();
    drop(guard1);

    let _guard2 = lock2.lock().unwrap();
}

#[test]
fn test_lock_file_creation() {
    let filename = "test_creation.lock";
    let mut lock = FileLock::new(filename);

    let guard = lock.lock().unwrap();
    assert!(std::path::Path::new(filename).exists());
    drop(guard);
}

#[test]
fn test_manual_unlock() {
    let mut lock = FileLock::new("test_manual.lock");
    let guard = lock.lock().unwrap();

    guard.unlock().unwrap();

    let _guard2 = lock.lock().unwrap();
}

#[test]
fn test_lock_file_cleanup() {
    let filename = "test_cleanup.lock";

    // Ensure the file doesn't exist initially
    let _ = std::fs::remove_file(filename);

    {
        let mut lock = FileLock::new(filename);
        let _guard = lock.lock().unwrap();

        // Verify the lock file exists while locked
        assert!(std::path::Path::new(filename).exists(), "Lock file should exist while locked");
    } // lock goes out of scope here, should be dropped and file deleted

    // Verify the lock file is deleted after FileLock is dropped
    assert!(!std::path::Path::new(filename).exists(), "Lock file should be deleted after FileLock is dropped");
}
