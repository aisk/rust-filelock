use filelock::FileLock;
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn test_concurrent_access() {
    let filename = "test_concurrent.lock";
    let barrier = Arc::new(Barrier::new(2));
    let results = Arc::new(std::sync::Mutex::new(Vec::new()));

    let barrier_clone = barrier.clone();
    let results_clone = results.clone();

    let handle1 = thread::spawn(move || {
        let mut lock = FileLock::new(filename);
        barrier_clone.wait();

        let guard = lock.lock().unwrap();
        results_clone.lock().unwrap().push("thread1 locked");
        thread::sleep(std::time::Duration::from_millis(100));
        drop(guard);
        results_clone.lock().unwrap().push("thread1 unlocked");
    });

    let barrier_clone = barrier.clone();
    let results_clone = results.clone();

    let handle2 = thread::spawn(move || {
        let mut lock = FileLock::new(filename);
        barrier_clone.wait();

        thread::sleep(std::time::Duration::from_millis(50));
        let guard = lock.lock().unwrap();
        results_clone.lock().unwrap().push("thread2 locked");
        drop(guard);
        results_clone.lock().unwrap().push("thread2 unlocked");
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    let results = results.lock().unwrap();
    assert_eq!(results.len(), 4);
    assert_eq!(results[0], "thread1 locked");
    assert_eq!(results[1], "thread1 unlocked");
    assert_eq!(results[2], "thread2 locked");
    assert_eq!(results[3], "thread2 unlocked");
}

#[test]
fn test_exclusive_lock() {
    let filename = "test_exclusive.lock";
    let counter = Arc::new(std::sync::Mutex::new(0));
    let mut handles = vec![];

    for _ in 0..3 {
        let counter_clone = counter.clone();
        let handle = thread::spawn(move || {
            let mut lock = FileLock::new(filename);
            let _guard = lock.lock().unwrap();

            let mut count = counter_clone.lock().unwrap();
            *count += 1;
            thread::sleep(std::time::Duration::from_millis(50));

            assert_eq!(*count, 1);
            *count -= 1;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(*counter.lock().unwrap(), 0);
}