use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

fn id(p: &Path) -> io::Result<(u64, u64)> {
    fs::metadata(p).map(|inode| (inode.dev(), inode.ino()))
}

fn main() -> io::Result<()> {
    let dir = fs::canonicalize(std::env::args_os().nth(1).expect("private run directory"))?;
    let lock_path = dir.join("home/volumes/registry.lock");
    let lock = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)?;
    lock.lock()?;
    let target = id(&lock_path)?;
    let executable = id(&dir.join("install/paired-journal"))?;
    let mut record = File::create_new(dir.join("evidence/contended-launch.txt"))?;
    let mut child = Command::new("bash")
        .arg(dir.join("run.sh"))
        .current_dir(&dir)
        .spawn()?;
    let start = Instant::now();
    let mut waiter = None;
    while start.elapsed() < Duration::from_secs(30) && child.try_wait()?.is_none() {
        for item in fs::read_dir("/proc")?.flatten() {
            let proc = item.path();
            if !item
                .file_name()
                .as_encoded_bytes()
                .iter()
                .all(u8::is_ascii_digit)
                || id(&proc.join("exe")).ok() != Some(executable)
            {
                continue;
            }
            if let Ok(descriptors) = fs::read_dir(proc.join("fd")) {
                for fd in descriptors.flatten() {
                    if id(&fd.path()).ok() == Some(target) {
                        waiter = Some((item.file_name(), fd.file_name()));
                        break;
                    }
                }
            }
            if waiter.is_some() {
                break;
            }
        }
        if waiter.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    if let Some((pid, fd)) = &waiter {
        std::thread::sleep(Duration::from_millis(100));
        let base = Path::new("/proc").join(pid);
        if id(&base.join("exe")).ok() != Some(executable)
            || id(&base.join("fd").join(fd)).ok() != Some(target)
        {
            waiter = None;
        }
    }
    drop(lock);
    writeln!(
        record,
        "held original registry inode {:?}; matching original driver and fd at both observations: {:?}; released after {:?}",
        target,
        waiter,
        start.elapsed()
    )?;
    let status = child.wait()?;
    writeln!(record, "driver exited {status}")?;
    record.sync_all()?;
    if waiter.is_none() || !status.success() {
        return Err(io::Error::other(
            "missing overlapping original driver or successful driver retirement",
        ));
    }
    Ok(())
}
