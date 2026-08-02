mod common;

use common::TestDir;
use filelock::FileLock;

#[test]
fn test_new_creates_lock_file() {
    let test_dir = TestDir::new("new");
    let filename = test_dir.path("test.lock");

    let lock = FileLock::new(&filename).unwrap();
    assert!(filename.exists(), "new() should create the lock file");

    drop(lock);
}

#[test]
fn test_basic_lock_and_unlock() {
    let test_dir = TestDir::new("basic");
    let mut lock = FileLock::new(test_dir.path("test.lock")).unwrap();

    // Test manual unlock
    let guard = lock.lock().unwrap();
    guard.unlock().unwrap();

    // Test RAII-style unlock
    let _guard = lock.lock().unwrap();
    // Guard is automatically dropped when it goes out of scope
}

#[test]
fn test_raii_automatic_unlock() {
    let test_dir = TestDir::new("raii");
    let mut lock = FileLock::new(test_dir.path("test.lock")).unwrap();

    {
        let _guard = lock.lock().unwrap();
        // Lock is held within this scope
    }

    // Lock should be automatically released, we can acquire it again
    let _guard = lock.lock().unwrap();
}

#[test]
fn test_try_lock_when_available() {
    let test_dir = TestDir::new("try-available");
    let mut lock = FileLock::new(test_dir.path("test.lock")).unwrap();

    let guard = lock.try_lock().unwrap();
    assert!(guard.is_some());
}

#[test]
fn test_multiple_instances_same_file() {
    let test_dir = TestDir::new("instances");
    let filename = test_dir.path("test.lock");
    let mut lock1 = FileLock::new(&filename).unwrap();
    let mut lock2 = FileLock::new(&filename).unwrap();

    // First instance acquires lock
    let guard1 = lock1.lock().unwrap();
    drop(guard1); // Release the lock

    // Second instance can now acquire the lock
    let _guard2 = lock2.lock().unwrap();
}

#[test]
fn test_lock_file_persists_after_release() {
    let test_dir = TestDir::new("persistence");
    let filename = test_dir.path("test.lock");

    {
        let mut lock = FileLock::new(&filename).unwrap();
        let _guard = lock.lock().unwrap();

        // Verify the lock file exists while locked
        assert!(filename.exists(), "Lock file should exist while locked");
    } // Guard is dropped, lock is released

    // The lock file is intentionally kept after release: deleting it would
    // break cross-process mutual exclusion (a concurrent process would create
    // a fresh inode and lock it independently).
    assert!(filename.exists(), "Lock file should be kept after release");
}

#[test]
fn test_from_file_and_accessors() {
    use std::io::{Read, Seek, Write};

    let test_dir = TestDir::new("from-file");
    let filename = test_dir.path("test.lock");
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&filename)
        .unwrap();

    let mut lock = FileLock::from_file(file);
    {
        let guard = lock.lock().unwrap();
        // Auxiliary data can be written to the lock file while it is held.
        guard.file().write_all(b"pid").unwrap();
    }

    let mut file = lock.into_file();
    file.rewind().unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    assert_eq!(contents, "pid");
}
