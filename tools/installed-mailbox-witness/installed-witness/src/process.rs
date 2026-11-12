use crate::ProcessSample;
use anyhow::{Context, Result, ensure};
use rustix::{event::{poll, PollFd, PollFlags, Timespec}, fd::OwnedFd, process::{Pid, PidfdFlags, pidfd_open}};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, os::unix::fs::MetadataExt, path::{Path, PathBuf}, time::Instant};

pub struct HeldExecutable {
    file: fs::File,
    pub digest: String,
}
impl HeldExecutable {
    pub fn open(path: &Path) -> Result<Self> {
        let file = fs::File::open(path)?;
        let metadata = file.metadata()?;
        ensure!(metadata.is_file() && metadata.len() <= 128 * 1024 * 1024, "executable measurement bound");
        let mut input = (&file).take(metadata.len() + 1);
        let mut hash = Sha256::new();
        let mut bytes = [0u8; 65536];
        let mut total = 0;
        loop {
            let n = input.read(&mut bytes)?;
            if n == 0 { break; }
            total += n as u64;
            hash.update(&bytes[..n]);
        }
        let after = file.metadata()?;
        ensure!(total == metadata.len() && identity(&metadata) == identity(&after) && metadata.mtime() == after.mtime() && metadata.mtime_nsec() == after.mtime_nsec() && metadata.ctime() == after.ctime() && metadata.ctime_nsec() == after.ctime_nsec(), "executable changed during capture");
        Ok(Self { file, digest: format!("{:x}", hash.finalize()) })
    }
    pub fn require_protected(&self) -> Result<()> {
        let metadata = self.file.metadata()?;
        let uid = metadata.uid();
        ensure!((uid == 0 || uid == rustix::process::geteuid().as_raw())
            && metadata.mode() & 0o022 == 0
            && metadata.mode() & 0o111 != 0
            && metadata.nlink() == 1,
            "expected helper must be root/current-user owned, executable, singly linked and protected from group/other writes");
        Ok(())
    }
    pub fn matches_inode(&self, dev: u64, ino: u64) -> Result<bool> {
        let m = self.file.metadata()?;
        Ok(m.dev() == dev && m.ino() == ino)
    }
    fn same_file(&self, other: &Self) -> Result<bool> {
        Ok(identity(&self.file.metadata()?) == identity(&other.file.metadata()?) && self.digest == other.digest)
    }
}
fn identity(metadata: &fs::Metadata) -> (u64, u64, u64) { (metadata.dev(), metadata.ino(), metadata.len()) }

#[derive(Serialize)]
pub struct ProcessObservation {
    pub pid: u32,
    pub executable: PathBuf,
    pub executable_sha256: String,
    pub initial: ProcessSample,
    pub before_stop: Option<ProcessSample>,
    pub before_stop_error: Option<String>,
    pub final_status: String,
    #[serde(skip)]
    pidfd: OwnedFd,
    #[serde(skip)]
    _executable: HeldExecutable,
}
impl ProcessObservation {
    pub fn capture(path: &Path, expected: Option<&HeldExecutable>) -> Result<Self> {
        let pid = fs::read_to_string(path)?.trim().parse::<u32>()?;
        Self::capture_pid(pid, expected)
    }
    pub fn capture_pid(pid: u32, expected: Option<&HeldExecutable>) -> Result<Self> {
        let stat = || -> Result<ProcessSample> { ProcessSample::parse(pid, &fs::read_to_string(format!("/proc/{pid}/stat"))?) };
        let initial = stat()?;
        ensure!(initial.state != 'Z', "observed process already exited");
        let raw = i32::try_from(pid)?;
        let pidfd = pidfd_open(Pid::from_raw(raw).context("invalid pid")?, PidfdFlags::empty()).context("pidfd observation required; no timer fallback")?;
        let exe_path = PathBuf::from(format!("/proc/{pid}/exe"));
        let executable = fs::read_link(&exe_path)?;
        let held = HeldExecutable::open(&exe_path)?;
        let rechecked = HeldExecutable::open(&exe_path)?;
        ensure!(held.same_file(&rechecked)? && stat()?.start_ticks == initial.start_ticks, "process changed during capture");
        if let Some(expected) = expected { expected.require_protected()?; ensure!(held.same_file(expected)?, "unexpected helper executable identity or digest"); }
        Ok(Self { pid, executable, executable_sha256: held.digest.clone(), initial, before_stop: None, before_stop_error: None, final_status: "not_observed".into(), pidfd, _executable: held })
    }
    pub fn sample_before_stop(&mut self) -> Result<()> {
        let checked = (|| {
            let sample = self.current()?;
            if sample.is_some() {
                let actual = HeldExecutable::open(Path::new(&format!("/proc/{}/exe", self.pid)))?;
                ensure!(self._executable.same_file(&actual)?, "original process changed executable before CPU sampling");
                ensure!(self.current()?.is_some(), "original process changed during CPU sampling");
            }
            Ok::<_, anyhow::Error>(sample)
        })();
        match checked {
            Ok(sample) => { self.before_stop = sample; Ok(()) }
            Err(error) => { self.before_stop_error = Some(format!("{error:#}")); Err(error) }
        }
    }
    fn current(&self) -> Result<Option<ProcessSample>> {
        match fs::read_to_string(format!("/proc/{}/stat", self.pid)) {
            Ok(raw) => {
                let sample = ProcessSample::parse(self.pid, &raw)?;
                Ok((sample.start_ticks == self.initial.start_ticks).then_some(sample))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
    pub fn observe_exit(&mut self, until: Instant) -> Result<()> {
        loop {
            let left = until.saturating_duration_since(Instant::now());
            ensure!(!left.is_zero(), "process-exit observation deadline");
            let timeout = Timespec::try_from(left)?;
            let mut fds = [PollFd::new(&self.pidfd, PollFlags::IN)];
            match poll(&mut fds, Some(&timeout)) {
                Err(rustix::io::Errno::INTR) => continue,
                outcome => { ensure!(outcome? > 0 && fds[0].revents().contains(PollFlags::IN), "process-exit observation timed out or invalid"); }
            }
            match self.current()? {
                None => { self.final_status = "original_process_exited_and_absent_or_pid_reused".into(); return Ok(()); }
                Some(sample) => {
                    self.final_status = if sample.state == 'Z' { "exited_not_reaped" } else { "inconsistent_exit_observation" }.into();
                    anyhow::bail!("pidfd exited but original process {} remains {}", self.pid, sample.state);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_pidfd_is_armed_before_stop_and_requires_reaped_process() {
        let directory = std::env::temp_dir().join(format!("spiralcore-held-helper-test-{}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let binary = directory.join("sleep");
        std::fs::copy("/bin/sleep", &binary).unwrap();
        let mut child = std::process::Command::new(&binary).arg("30").spawn().unwrap();
        let expected = HeldExecutable::open(&binary).unwrap();
        let observed = ProcessObservation::capture_pid(child.id(), Some(&expected));
        child.kill().unwrap();
        child.wait().unwrap();
        std::fs::remove_dir_all(directory).unwrap();
        let mut observed = observed.unwrap();
        observed.observe_exit(Instant::now() + std::time::Duration::from_secs(1)).unwrap();
        assert_eq!(observed.final_status, "original_process_exited_and_absent_or_pid_reused");
    }
    #[test]
    fn process_executable_must_match_the_held_expected_file_and_digest() {
        let mut child = std::process::Command::new("/bin/sleep").arg("30").spawn().unwrap();
        let foreign = HeldExecutable::open(&std::env::current_exe().unwrap()).unwrap();
        let result = ProcessObservation::capture_pid(child.id(), Some(&foreign));
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(result.is_err());
    }
    #[test]
    fn same_pid_exec_change_is_not_silently_recorded_as_original_cpu() {
        use std::io::{Read, Write};
        use std::process::Stdio;
        let mut child = std::process::Command::new("/bin/sh")
            .args(["-c", "read marker; exec /bin/cat"])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        let mut observed = ProcessObservation::capture_pid(child.id(), None).unwrap();
        child.stdin.as_mut().unwrap().write_all(b"go\nx").unwrap();
        let mut byte = [0];
        child.stdout.as_mut().unwrap().read_exact(&mut byte).unwrap();
        let result = observed.sample_before_stop();
        child.kill().unwrap();
        child.wait().unwrap();
        assert_eq!(byte, [b'x']);
        assert!(result.is_err(), "a different executable reused the same PID/starttime");
        assert!(observed.before_stop_error.is_some());
    }

    #[test]
    fn writable_expected_helper_cannot_authorize_matching_process() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("spiralcore-writable-helper-test-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("sleep");
        // Avoid an accelerated copy's transient writer ownership at exec. Finish
        // this fixture's explicit writer before executing the protected image.
        {
            use std::io::Write;
            let bytes = std::fs::read("/bin/sleep").unwrap();
            let mut output = std::fs::OpenOptions::new().write(true).create_new(true).open(&path).unwrap();
            output.write_all(&bytes).unwrap();
            output.sync_all().unwrap();
        }
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut child = std::process::Command::new(&path).arg("30").spawn().unwrap();
        // Command::spawn has completed exec. Changing this exact live inode's
        // mode now tests writable metadata without racing executable setup.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o777)).unwrap();
        let expected = HeldExecutable::open(&path).unwrap();
        let actual = HeldExecutable::open(Path::new(&format!("/proc/{}/exe", child.id()))).unwrap();
        assert!(expected.same_file(&actual).unwrap(), "fixture must match the live executable");
        let observed = ProcessObservation::capture_pid(child.id(), Some(&expected));
        child.kill().unwrap();
        child.wait().unwrap();
        std::fs::remove_dir_all(dir).unwrap();
        assert!(observed.is_err(), "a writable expected helper cannot authorize capture");
    }

}
