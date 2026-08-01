mod common;

use common::TestDir;
use filelock::FileLock;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const CHILD_MODE: &str = "FILELOCK_PROCESS_TEST_CHILD";
const LOCK_PATH: &str = "FILELOCK_PROCESS_TEST_PATH";

#[test]
fn test_exclusive_lock_across_processes() {
    if std::env::var_os(CHILD_MODE).is_some() {
        run_holder_process();
        return;
    }

    let test_dir = TestDir::new("process");
    let path = test_dir.path("test.lock");
    let mut holder = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "test_exclusive_lock_across_processes",
            "--nocapture",
        ])
        .env(CHILD_MODE, "1")
        .env(LOCK_PATH, &path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let stdout = holder.stdout.take().unwrap();
    let (locked_tx, locked_rx) = mpsc::channel();
    let output_reader = thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if line.unwrap() == "LOCKED" {
                let _ = locked_tx.send(());
            }
        }
    });

    if locked_rx.recv_timeout(Duration::from_secs(5)).is_err() {
        let _ = holder.kill();
        let _ = holder.wait();
        output_reader.join().unwrap();
        panic!("holder process did not acquire the file lock");
    }

    let mut contender = FileLock::new(&path);
    let unavailable_while_held = contender.try_lock().unwrap().is_none();

    writeln!(holder.stdin.take().unwrap(), "RELEASE").unwrap();
    let status = holder.wait().unwrap();
    output_reader.join().unwrap();

    let available_after_release = contender.try_lock().unwrap().is_some();
    assert!(status.success(), "holder process failed: {status}");
    assert!(
        unavailable_while_held,
        "another process acquired an already-held file lock"
    );
    assert!(
        available_after_release,
        "file lock remained unavailable after the holder exited"
    );
}

fn run_holder_process() {
    let path = std::env::var_os(LOCK_PATH).unwrap();
    let mut lock = FileLock::new(path);
    let _guard = lock.lock().unwrap();

    println!("LOCKED");
    std::io::stdout().flush().unwrap();

    let mut command = String::new();
    std::io::stdin().read_line(&mut command).unwrap();
    assert_eq!(command.trim(), "RELEASE");
}
