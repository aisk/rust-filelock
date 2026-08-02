#![cfg(feature = "tokio")]

mod common;

use common::TestDir;
use filelock::FileLock;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
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
        let _ = release_rx.recv_timeout(Duration::from_secs(2));
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    (release_tx, holder)
}

#[tokio::test]
async fn async_lock_yields_while_contended() {
    let test_dir = TestDir::new("tokio-yields");
    let path = test_dir.path("test.lock");
    let (release_holder, holder) = hold_lock(path.clone());
    let mut contender = FileLock::new(path).unwrap();

    let started = Instant::now();
    let result = tokio::time::timeout(Duration::from_millis(30), contender.lock_async()).await;
    release_holder.send(()).unwrap();

    assert!(result.is_err(), "the contended lock unexpectedly completed");
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "lock_async blocked the runtime worker instead of yielding"
    );
    holder.join().unwrap();
}

#[tokio::test]
async fn async_lock_acquires_after_release() {
    let test_dir = TestDir::new("tokio-acquire");
    let path = test_dir.path("test.lock");
    let (release_holder, holder) = hold_lock(path.clone());
    let mut contender = FileLock::new(path).unwrap();
    let release = async move {
        tokio::time::sleep(Duration::from_millis(30)).await;
        release_holder.send(()).unwrap();
    };

    let ((), result) = tokio::join!(release, async {
        tokio::time::timeout(Duration::from_secs(1), contender.lock_async()).await
    });
    let guard = result.expect("timed out waiting for the lock").unwrap();
    drop(guard);
    holder.join().unwrap();
}

#[tokio::test]
async fn cancelled_async_lock_does_not_acquire_later() {
    let test_dir = TestDir::new("tokio-cancel");
    let path = test_dir.path("test.lock");
    let (release_holder, holder) = hold_lock(path.clone());
    let waiter_path = path.clone();
    let waiter = tokio::spawn(async move {
        let mut lock = FileLock::new(waiter_path).unwrap();
        let _guard = lock.lock_async().await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(10)).await;
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    release_holder.send(()).unwrap();
    holder.join().unwrap();

    let mut contender = FileLock::new(path).unwrap();
    let _guard = contender
        .try_lock()
        .unwrap()
        .expect("a cancelled async waiter acquired the lock later");
}

#[tokio::test]
async fn aborting_a_lock_holder_releases_the_lock() {
    let test_dir = TestDir::new("tokio-abort-holder");
    let path = test_dir.path("test.lock");
    let holder_path = path.clone();
    let (acquired_tx, acquired_rx) = tokio::sync::oneshot::channel();
    let holder = tokio::spawn(async move {
        let mut lock = FileLock::new(holder_path).unwrap();
        let _guard = lock.lock_async().await.unwrap();
        acquired_tx.send(()).unwrap();
        std::future::pending::<()>().await;
    });

    acquired_rx.await.unwrap();
    let mut contender = FileLock::new(path).unwrap();
    assert!(contender.try_lock().unwrap().is_none());

    holder.abort();
    assert!(holder.await.unwrap_err().is_cancelled());
    let _guard = contender
        .try_lock()
        .unwrap()
        .expect("aborting the holder did not release its lock");
}

#[tokio::test]
async fn owned_async_guard_can_move_into_spawned_task() {
    let test_dir = TestDir::new("tokio-owned");
    let path = test_dir.path("test.lock");

    let guard = FileLock::new(&path)
        .unwrap()
        .lock_owned_async()
        .await
        .unwrap();

    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let holder = tokio::spawn(async move {
        let _guard = guard;
        release_rx.await.unwrap();
    });

    let mut contender = FileLock::new(&path).unwrap();
    assert!(
        contender.try_lock().unwrap().is_none(),
        "the lock was not held by the guard moved into the task"
    );

    release_tx.send(()).unwrap();
    holder.await.unwrap();
    assert!(
        contender.try_lock().unwrap().is_some(),
        "dropping the moved guard did not release the lock"
    );
}

#[tokio::test]
async fn multiple_async_waiters_remain_exclusive_and_make_progress() {
    let test_dir = TestDir::new("tokio-waiters");
    let path = test_dir.path("test.lock");
    let inside = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));
    let mut waiters = Vec::new();

    for _ in 0..4 {
        let waiter_path = path.clone();
        let inside = Arc::clone(&inside);
        let completed = Arc::clone(&completed);
        waiters.push(tokio::spawn(async move {
            let mut lock = FileLock::new(waiter_path).unwrap();
            let _guard = lock.lock_async().await.unwrap();
            assert_eq!(inside.fetch_add(1, Ordering::SeqCst), 0);
            tokio::time::sleep(Duration::from_millis(5)).await;
            assert_eq!(inside.fetch_sub(1, Ordering::SeqCst), 1);
            completed.fetch_add(1, Ordering::SeqCst);
        }));
    }

    for waiter in waiters {
        tokio::time::timeout(Duration::from_secs(2), waiter)
            .await
            .expect("an async waiter made no progress")
            .unwrap();
    }
    assert_eq!(completed.load(Ordering::SeqCst), 4);
}
