mod common;

use common::TestDir;
use filelock::FileLock;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

fn hold_lock(path: std::path::PathBuf) -> (mpsc::Sender<()>, thread::JoinHandle<()>) {
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let holder = thread::spawn(move || {
        let mut lock = FileLock::new(path).unwrap();
        let _guard = lock.lock().unwrap();
        ready_tx.send(()).unwrap();
        let _ = release_rx.recv_timeout(Duration::from_secs(5));
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    (release_tx, holder)
}

#[test]
fn test_lock_timeout_acquires_when_available() {
    let test_dir = TestDir::new("timeout-available");
    let mut lock = FileLock::new(test_dir.path("test.lock")).unwrap();

    let guard = lock.lock_timeout(Duration::ZERO).unwrap();
    assert!(guard.is_some(), "an uncontended lock should be acquired");
}

#[test]
fn test_lock_timeout_expires_while_held() {
    let test_dir = TestDir::new("timeout-expires");
    let path = test_dir.path("test.lock");
    let (release_holder, holder) = hold_lock(path.clone());
    let mut contender = FileLock::new(path).unwrap();

    let started = Instant::now();
    let result = contender.lock_timeout(Duration::from_millis(50)).unwrap();
    let elapsed = started.elapsed();
    release_holder.send(()).unwrap();
    holder.join().unwrap();

    assert!(
        result.is_none(),
        "the contended lock unexpectedly completed"
    );
    assert!(
        elapsed >= Duration::from_millis(50),
        "lock_timeout returned before the timeout elapsed ({elapsed:?})"
    );
}

#[test]
fn test_lock_timeout_acquires_after_release() {
    let test_dir = TestDir::new("timeout-acquires");
    let path = test_dir.path("test.lock");
    let (release_holder, holder) = hold_lock(path.clone());
    let mut contender = FileLock::new(path).unwrap();

    let releaser = thread::spawn(move || {
        thread::sleep(Duration::from_millis(30));
        release_holder.send(()).unwrap();
    });

    let guard = contender.lock_timeout(Duration::from_secs(10)).unwrap();
    assert!(
        guard.is_some(),
        "the lock was not acquired after the holder released it"
    );

    drop(guard);
    releaser.join().unwrap();
    holder.join().unwrap();
}

#[test]
fn test_lock_shared_timeout() {
    let test_dir = TestDir::new("timeout-shared");
    let path = test_dir.path("test.lock");
    let mut reader1 = FileLock::new(&path).unwrap();
    let mut reader2 = FileLock::new(&path).unwrap();

    let _guard1 = reader1.lock_shared().unwrap();
    let guard2 = reader2.lock_shared_timeout(Duration::ZERO).unwrap();
    assert!(
        guard2.is_some(),
        "a shared lock should be granted while another shared lock is held"
    );
}
