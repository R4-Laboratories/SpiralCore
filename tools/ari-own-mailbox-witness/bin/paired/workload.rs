use super::fixture::{BODY_LEN, CALL_INPUT_LIMIT};
use anyhow::{Context, Result, ensure};
use mvm_client::entrypoint::dispatch::{self, CapturedOutput, DispatchStdin, EntrypointDispatch};
use serde_json::Value;
use std::{
    io::{self, Read, Write},
    os::fd::AsRawFd,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
const CALL_OUTPUT_LIMIT: usize = if cfg!(feature = "large-object-fixture") {
    2 * BODY_LEN + 1024
} else {
    4096
};
pub fn dispatch(name: &str) -> Result<()> {
    ensure!([super::A, super::B].contains(&name), "invalid workload");
    let mut input = Vec::new();
    std::io::stdin()
        .take((CALL_INPUT_LIMIT + 1) as u64)
        .read_to_end(&mut input)?;
    ensure!(input.len() <= CALL_INPUT_LIMIT, "oversized fixture Call");
    let mut output = CapturedOutput::default();
    let completed = dispatch::dispatch(
        EntrypointDispatch {
            vm_name: name,
            stdin: DispatchStdin::OneShot(input),
            timeout_secs: 35,
            session_id: None,
        },
        &mut output,
    )?;
    ensure!(
        completed.exit_code() == 0 && !output.truncated && output.stdout.len() <= CALL_OUTPUT_LIMIT,
        "original workload ended {:?}: {}",
        completed.terminal,
        String::from_utf8_lossy(&output.stderr[..output.stderr.len().min(400)])
    );
    std::io::stdout().write_all(&output.stdout)?;
    Ok(())
}
// Both pipes stay with this owned child. The absolute deadline also covers
// a child that stalls before reading its request or closing its response.
fn nonblocking(pipe: &impl AsRawFd) -> io::Result<()> {
    let fd = pipe.as_raw_fd();
    // SAFETY: pipe owns a live descriptor for the duration of both calls.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub struct Call {
    child: Child,
    deadline: Instant,
    output: Vec<u8>,
    eof: bool,
    done: bool,
    pub input_bytes: usize,
    pub output_bytes: usize,
}
impl Call {
    pub fn start(name: &str, payload: &Value, deadline: Instant) -> Result<Self> {
        let bytes = serde_json::to_vec(payload)?;
        ensure!(bytes.len() <= CALL_INPUT_LIMIT, "oversized fixture Call");
        let child = Command::new(std::env::current_exe()?)
            .args(["call", name])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        Self::submit(child, &bytes, deadline)
    }

    fn submit(child: Child, bytes: &[u8], deadline: Instant) -> Result<Self> {
        let mut call = Self {
            child,
            deadline,
            output: Vec::new(),
            eof: false,
            done: false,
            input_bytes: bytes.len(),
            output_bytes: 0,
        };
        let mut input = call.child.stdin.take().context("missing worker stdin")?;
        nonblocking(&input)?;
        nonblocking(call.child.stdout.as_ref().context("missing guest result")?)?;
        let mut remaining = bytes;
        while !remaining.is_empty() {
            call.drain()?;
            match input.write(remaining) {
                Ok(0) => anyhow::bail!("original worker closed its input"),
                Ok(written) => remaining = &remaining[written..],
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => call.pause()?,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(call)
    }

    fn pause(&self) -> Result<()> {
        let left = self
            .deadline
            .checked_duration_since(Instant::now())
            .context("original guest Call timed out")?;
        std::thread::sleep(left.min(Duration::from_millis(5)));
        Ok(())
    }

    fn drain(&mut self) -> Result<()> {
        let mut chunk = [0; 4096];
        loop {
            ensure!(
                Instant::now() < self.deadline,
                "original guest Call timed out"
            );
            if self.eof {
                return Ok(());
            }
            match self
                .child
                .stdout
                .as_mut()
                .context("missing guest result")?
                .read(&mut chunk)
            {
                Ok(0) => self.eof = true,
                Ok(n) => {
                    ensure!(
                        n <= CALL_OUTPUT_LIMIT - self.output.len(),
                        "oversized guest result"
                    );
                    self.output.extend_from_slice(&chunk[..n]);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            }
        }
    }

    pub fn wait(&mut self) -> Result<Value> {
        loop {
            if let Some(result) = self.completed()? {
                return Ok(result);
            }
            self.pause()?;
        }
    }
    pub fn completed(&mut self) -> Result<Option<Value>> {
        if self.done {
            return Ok(None);
        }
        self.drain()?;
        if let Some(exit) = self.child.try_wait()? {
            ensure!(exit.success(), "original guest Call failed");
            if self.eof {
                self.done = true;
                self.output_bytes = self.output.len();
                return Ok(Some(serde_json::from_slice(&self.output)?));
            }
        }
        Ok(None)
    }
}
impl Drop for Call {
    fn drop(&mut self) {
        if !self.done {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[cfg(test)]
mod pipe_tests {
    use super::*;
    #[test]
    fn original_call_drains_both_pipes_and_checks_maximum() {
        let cat = || {
            Command::new("cat")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap()
        };
        let until = Instant::now() + Duration::from_secs(2);
        let input = serde_json::to_vec(&"x".repeat(CALL_OUTPUT_LIMIT - 2)).unwrap();
        let mut call = Call::submit(cat(), &input, until).unwrap();
        assert_eq!(
            call.wait().unwrap().as_str().unwrap().len(),
            CALL_OUTPUT_LIMIT - 2
        );
        let mut oversized = input;
        oversized.push(b' ');
        let result = Call::submit(cat(), &oversized, until).and_then(|mut call| call.wait());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("oversized guest result")
        );
    }
    #[test]
    fn stalled_original_cannot_block_large_input_past_deadline() {
        let child = Command::new("sleep")
            .arg("5")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let start = Instant::now();
        let err = Call::submit(
            child,
            &vec![0; 1024 * 1024],
            start + Duration::from_millis(100),
        )
        .err()
        .unwrap();
        assert!(err.to_string().contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(3));
    }
}
