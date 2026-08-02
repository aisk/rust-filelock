mod common;

use common::TestDir;
use filelock::FileLock;

#[test]
fn test_shared_locks_can_be_held_concurrently() {
    let test_dir = TestDir::new("shared-concurrent");
    let filename = test_dir.path("test.lock");
    let mut lock1 = FileLock::new(&filename).unwrap();
    let mut lock2 = FileLock::new(&filename).unwrap();

    let _guard1 = lock1.lock_shared().unwrap();
    let guard2 = lock2
        .try_lock_shared()
        .unwrap()
        .expect("a second shared lock should be granted while one is held");
    drop(guard2);
}

#[test]
fn test_shared_lock_blocks_exclusive() {
    let test_dir = TestDir::new("shared-blocks-exclusive");
    let filename = test_dir.path("test.lock");
    let mut reader = FileLock::new(&filename).unwrap();
    let mut writer = FileLock::new(&filename).unwrap();

    let guard = reader.lock_shared().unwrap();
    assert!(
        writer.try_lock().unwrap().is_none(),
        "an exclusive lock was granted while a shared lock was held"
    );

    drop(guard);
    assert!(
        writer.try_lock().unwrap().is_some(),
        "the exclusive lock remained unavailable after the shared lock was released"
    );
}

#[test]
fn test_exclusive_lock_blocks_shared() {
    let test_dir = TestDir::new("exclusive-blocks-shared");
    let filename = test_dir.path("test.lock");
    let mut writer = FileLock::new(&filename).unwrap();
    let mut reader = FileLock::new(&filename).unwrap();

    let guard = writer.lock().unwrap();
    assert!(
        reader.try_lock_shared().unwrap().is_none(),
        "a shared lock was granted while an exclusive lock was held"
    );

    drop(guard);
    assert!(
        reader.try_lock_shared().unwrap().is_some(),
        "the shared lock remained unavailable after the exclusive lock was released"
    );
}
