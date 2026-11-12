use anyhow::{Context, Result, ensure};
use mvm_client::entrypoint::dispatch::{self, CapturedOutput, DispatchStdin, EntrypointDispatch};
use serde_json::Value;
use std::{
    io::{Read, Write},
    process::{Child, Command, Stdio},
};
pub fn dispatch(name: &str) -> Result<()> {
    ensure!([super::A, super::B].contains(&name), "invalid workload");
    let mut input = Vec::new();
    std::io::stdin().take(2049).read_to_end(&mut input)?;
    ensure!(input.len() <= 2048, "oversized fixture Call");
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
        completed.exit_code() == 0 && !output.truncated && output.stdout.len() <= 4096,
        "original workload ended {:?}: {}",
        completed.terminal,
        String::from_utf8_lossy(&output.stderr[..output.stderr.len().min(400)])
    );
    std::io::stdout().write_all(&output.stdout)?;
    Ok(())
}
pub struct Call {
    child: Child,
    done: bool,
    pub input_bytes: usize,
    pub output_bytes: usize,
}
impl Call {
    pub fn start(name: &str, payload: &Value) -> Result<Self> {
        let bytes = serde_json::to_vec(payload)?;
        ensure!(bytes.len() <= 2048, "oversized fixture Call");
        let child = Command::new(std::env::current_exe()?)
            .args(["call", name])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let mut call = Self {
            child,
            done: false,
            input_bytes: bytes.len(),
            output_bytes: 0,
        };
        call.child
            .stdin
            .take()
            .context("missing worker stdin")?
            .write_all(&bytes)?;
        Ok(call)
    }
    pub fn wait(&mut self, until: std::time::Instant) -> Result<Value> {
        loop {
            if let Some(result) = self.completed()? {
                return Ok(result);
            }
            ensure!(
                std::time::Instant::now() < until,
                "original guest Call timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    pub fn completed(&mut self) -> Result<Option<Value>> {
        if self.done {
            return Ok(None);
        }
        if let Some(exit) = self.child.try_wait()? {
            self.done = true;
            ensure!(exit.success(), "original guest Call failed");
            let mut bytes = Vec::new();
            self.child
                .stdout
                .take()
                .context("missing guest result")?
                .take(4097)
                .read_to_end(&mut bytes)?;
            ensure!(bytes.len() <= 4096, "oversized guest result");
            self.output_bytes = bytes.len();
            Ok(Some(serde_json::from_slice(&bytes)?))
        } else {
            Ok(None)
        }
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
