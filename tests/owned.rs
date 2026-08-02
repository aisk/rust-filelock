mod common;

use common::TestDir;
use filelock::FileLock;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
fn test_owned_guard_can_move_across_threads() {
    let test_dir = TestDir::new("owned-move");
    let filename = test_dir.path("test.lock");

    let guard = FileLock::new(&filename).unwrap().lock_owned().unwrap();

    let (release_tx, release_rx) = mpsc::channel();
    let holder = thread::spawn(move || {
        let _guard = guard;
        release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    });

    let mut contender = FileLock::new(&filename).unwrap();
    assert!(
        contender.try_lock().unwrap().is_none(),
        "the lock was not held by the moved guard"
    );

    release_tx.send(()).unwrap();
    holder.join().unwrap();
    assert!(
        contender.try_lock().unwrap().is_some(),
        "dropping the moved guard did not release the lock"
    );
}

#[test]
fn test_owned_guard_unlock_returns_lock_for_reuse() {
    let test_dir = TestDir::new("owned-reuse");
    let filename = test_dir.path("test.lock");

    let guard = FileLock::new(&filename).unwrap().lock_owned().unwrap();
    let mut lock = guard.unlock().unwrap();

    // The returned FileLock can be locked again without reopening the file.
    let _guard = lock.lock().unwrap();
}

#[test]
fn test_owned_shared_guards() {
    let test_dir = TestDir::new("owned-shared");
    let filename = test_dir.path("test.lock");

    let guard1 = FileLock::new(&filename)
        .unwrap()
        .lock_shared_owned()
        .unwrap();
    let guard2 = FileLock::new(&filename)
        .unwrap()
        .lock_shared_owned()
        .unwrap();

    let mut writer = FileLock::new(&filename).unwrap();
    assert!(writer.try_lock().unwrap().is_none());

    drop(guard1);
    drop(guard2);
    assert!(writer.try_lock().unwrap().is_some());
}
