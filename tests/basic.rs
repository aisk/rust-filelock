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