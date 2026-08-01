use filelock::FileLock;

#[test]
fn test_new_does_not_create_lock_file() {
    let filename = std::env::temp_dir().join(format!("filelock-new-{}.lock", std::process::id()));
    let _ = std::fs::remove_file(&filename);

    let lock = FileLock::new(&filename);
    assert!(!filename.exists());

    drop(lock);
}

#[test]
fn test_basic_lock_and_unlock() {
    let filename = "test_basic.lock";
    let mut lock = FileLock::new(filename);

    // Test manual unlock
    let guard = lock.lock().unwrap();
    guard.unlock().unwrap();

    // Test RAII-style unlock
    let _guard = lock.lock().unwrap();
    // Guard is automatically dropped when it goes out of scope
}

#[test]
fn test_raii_automatic_unlock() {
    let filename = "test_raii.lock";
    let mut lock = FileLock::new(filename);

    {
        let _guard = lock.lock().unwrap();
        // Lock is held within this scope
    }

    // Lock should be automatically released, we can acquire it again
    let _guard = lock.lock().unwrap();
}

#[test]
fn test_try_lock_when_available() {
    let filename = "test_try_available.lock";
    let mut lock = FileLock::new(filename);

    let guard = lock.try_lock().unwrap();
    assert!(guard.is_some());
}

#[test]
fn test_multiple_instances_same_file() {
    let filename = "test_instances.lock";
    let mut lock1 = FileLock::new(filename);
    let mut lock2 = FileLock::new(filename);

    // First instance acquires lock
    let guard1 = lock1.lock().unwrap();
    drop(guard1); // Release the lock

    // Second instance can now acquire the lock
    let _guard2 = lock2.lock().unwrap();
}

#[test]
fn test_lock_file_persists_after_release() {
    let filename = "test_creation.lock";

    // Ensure the file doesn't exist initially
    let _ = std::fs::remove_file(filename);

    {
        let mut lock = FileLock::new(filename);
        let _guard = lock.lock().unwrap();

        // Verify the lock file exists while locked
        assert!(
            std::path::Path::new(filename).exists(),
            "Lock file should exist while locked"
        );
    } // Guard is dropped, lock is released

    // The lock file is intentionally kept after release: deleting it would
    // break cross-process mutual exclusion (a concurrent process would create
    // a fresh inode and lock it independently).
    assert!(
        std::path::Path::new(filename).exists(),
        "Lock file should be kept after release"
    );

    let _ = std::fs::remove_file(filename);
}
