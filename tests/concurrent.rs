mod common;

use common::TestDir;
use filelock::FileLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

#[cfg(unix)]
#[test]
fn test_lock_is_not_inherited_across_exec() {
    const CHILD_MODE: &str = "FILELOCK_CLOEXEC_TEST_CHILD";
    const LOCK_PATH: &str = "FILELOCK_CLOEXEC_TEST_PATH";

    if std::env::var_os(CHILD_MODE).is_some() {
        let path = std::env::var_os(LOCK_PATH).unwrap();
        let mut lock = FileLock::new(path).unwrap();
        let _guard = lock.lock().unwrap();

        std::process::Command::new("sleep")
            .arg("1")
            .spawn()
            .unwrap();
        std::process::exit(0);
    }

    let test_dir = TestDir::new("cloexec");
    let path = test_dir.path("test.lock");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "test_lock_is_not_inherited_across_exec",
            "--nocapture",
        ])
        .env(CHILD_MODE, "1")
        .env(LOCK_PATH, &path)
        .status()
        .unwrap();
    assert!(status.success());

    let mut contender = FileLock::new(&path).unwrap();
    let acquired = contender.try_lock().unwrap().is_some();
    assert!(
        acquired,
        "an exec child inherited and retained the file lock"
    );
}

#[test]
fn test_concurrent_lock_access() {
    let test_dir = TestDir::new("concurrent");
    let filename = test_dir.path("test.lock");
    let (holder_ready_tx, holder_ready_rx) = mpsc::channel();
    let (release_holder_tx, release_holder_rx) = mpsc::channel();
    let (waiter_acquired_tx, waiter_acquired_rx) = mpsc::channel();

    let holder_filename = filename.clone();
    let holder = thread::spawn(move || {
        let mut lock = FileLock::new(holder_filename).unwrap();
        let guard = lock.lock().unwrap();
        holder_ready_tx.send(()).unwrap();
        release_holder_rx.recv().unwrap();
        drop(guard);
    });

    holder_ready_rx
        .recv_timeout(Duration::from_secs(5))
        .unwrap();

    let waiter = thread::spawn(move || {
        let mut lock = FileLock::new(filename).unwrap();
        let _guard = lock.lock().unwrap();
        waiter_acquired_tx.send(()).unwrap();
    });

    let acquired_before_release = waiter_acquired_rx.recv_timeout(Duration::from_millis(100));
    release_holder_tx.send(()).unwrap();

    if acquired_before_release.is_err() {
        waiter_acquired_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
    }
    holder.join().unwrap();
    waiter.join().unwrap();

    assert_eq!(
        acquired_before_release,
        Err(mpsc::RecvTimeoutError::Timeout),
        "the waiter acquired the file lock before the holder released it"
    );
}

#[test]
fn test_exclusive_access_across_threads() {
    let test_dir = TestDir::new("exclusive");
    let filename = test_dir.path("test.lock");
    let concurrent_counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // Spawn multiple threads that all try to access the same file
    for i in 0..4 {
        let counter_clone = concurrent_counter.clone();
        let filename = filename.clone();
        let handle = thread::spawn(move || {
            let mut lock = FileLock::new(filename).unwrap();
            let _guard = lock.lock().unwrap();

            let previous = counter_clone.fetch_add(1, Ordering::SeqCst);
            assert_eq!(
                previous, 0,
                "Only one thread should be in critical section (thread {})",
                i
            );

            thread::sleep(Duration::from_millis(20));

            assert_eq!(counter_clone.fetch_sub(1, Ordering::SeqCst), 1);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // All threads should have completed successfully
    assert_eq!(concurrent_counter.load(Ordering::SeqCst), 0);
}

#[test]
fn test_try_lock_returns_immediately_when_held() {
    let test_dir = TestDir::new("try-contended");
    let filename = test_dir.path("test.lock");
    let (holder_ready_tx, holder_ready_rx) = mpsc::channel();
    let (release_holder_tx, release_holder_rx) = mpsc::channel();
    let holder_filename = filename.clone();
    let holder = thread::spawn(move || {
        let mut lock = FileLock::new(holder_filename).unwrap();
        let _guard = lock.lock().unwrap();
        holder_ready_tx.send(()).unwrap();
        release_holder_rx.recv().unwrap();
    });

    holder_ready_rx
        .recv_timeout(Duration::from_secs(5))
        .unwrap();

    let (result_tx, result_rx) = mpsc::channel();
    let contender_filename = filename.clone();
    let contender = thread::spawn(move || {
        let mut lock = FileLock::new(contender_filename).unwrap();
        let unavailable = lock.try_lock().unwrap().is_none();
        result_tx.send(unavailable).unwrap();
    });

    let result = result_rx.recv_timeout(Duration::from_secs(5));
    release_holder_tx.send(()).unwrap();
    holder.join().unwrap();
    contender.join().unwrap();

    assert!(result.unwrap(), "try_lock blocked on a held lock");

    let mut available = FileLock::new(filename).unwrap();
    assert!(available.try_lock().unwrap().is_some());
}
