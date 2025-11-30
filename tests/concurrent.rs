use filelock::FileLock;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn test_concurrent_lock_access() {
    let filename = "test_concurrent.lock";
    let barrier = Arc::new(Barrier::new(2));
    let access_log = Arc::new(Mutex::new(Vec::<&'static str>::new()));

    // Thread 1: Acquires lock first and holds it briefly
    let barrier_clone = barrier.clone();
    let log_clone = access_log.clone();
    let handle1 = thread::spawn(move || {
        let mut lock = FileLock::new(filename);
        barrier_clone.wait(); // Synchronize start

        let guard = lock.lock().unwrap();
        log_clone.lock().unwrap().push("thread1_acquired");
        thread::sleep(Duration::from_millis(100));
        drop(guard);
        log_clone.lock().unwrap().push("thread1_released");
    });

    // Thread 2: Tries to acquire same lock after a short delay
    let barrier_clone = barrier.clone();
    let log_clone = access_log.clone();
    let handle2 = thread::spawn(move || {
        let mut lock = FileLock::new(filename);
        barrier_clone.wait(); // Synchronize start

        thread::sleep(Duration::from_millis(50)); // Let thread1 acquire first
        let guard = lock.lock().unwrap();
        log_clone.lock().unwrap().push("thread2_acquired");
        drop(guard);
        log_clone.lock().unwrap().push("thread2_released");
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    let log = access_log.lock().unwrap();
    assert_eq!(log.len(), 4);
    assert_eq!(log[0], "thread1_acquired");
    assert_eq!(log[1], "thread1_released");
    assert_eq!(log[2], "thread2_acquired");
    assert_eq!(log[3], "thread2_released");
}

#[test]
fn test_exclusive_access_across_threads() {
    let filename = "test_exclusive.lock";
    let concurrent_counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    // Spawn multiple threads that all try to access the same file
    for i in 0..4 {
        let counter_clone = concurrent_counter.clone();
        let handle = thread::spawn(move || {
            let mut lock = FileLock::new(filename);
            let _guard = lock.lock().unwrap();

            // Critical section: only one thread should be here at a time
            {
                let mut count = counter_clone.lock().unwrap();
                *count += 1;

                // Simulate work
                thread::sleep(Duration::from_millis(20));

                // Verify exclusivity: counter should be exactly 1
                assert_eq!(*count, 1, "Only one thread should be in critical section (thread {})", i);

                *count -= 1;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // All threads should have completed successfully
    assert_eq!(*concurrent_counter.lock().unwrap(), 0);
}

#[test]
fn test_lock_timeout_behavior() {
    let filename = "test_timeout.lock";
    let holder_started = Arc::new(Barrier::new(2));
    let wait_duration = Duration::from_millis(200);

    let holder_started_clone = holder_started.clone();
    let holder_handle = thread::spawn(move || {
        let mut lock = FileLock::new(filename);
        let _guard = lock.lock().unwrap();
        holder_started_clone.wait(); // Signal that lock is held
        thread::sleep(wait_duration); // Hold the lock
    });

    let waiter_started_clone = holder_started.clone();
    let waiter_handle = thread::spawn(move || {
        let mut lock = FileLock::new(filename);
        waiter_started_clone.wait(); // Wait for holder to start

        let start_time = Instant::now();
        let _guard = lock.lock().unwrap();
        let elapsed = start_time.elapsed();

        // Should have waited at least as long as the holder held the lock
        assert!(elapsed >= wait_duration, "Waiter should have blocked for at least {:?}", wait_duration);
    });

    holder_handle.join().unwrap();
    waiter_handle.join().unwrap();
}
