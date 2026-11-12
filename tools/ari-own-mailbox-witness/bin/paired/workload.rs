use super::fixture::{BODY_LEN, CALL_INPUT_LIMIT};
use anyhow::{Context, Result, ensure};
use mvm_client::entrypoint::dispatch::{self, CapturedOutput, DispatchStdin, EntrypointDispatch};
use serde_json::Value;
use std::{
    io::{Read, Write},
    process::{Child, Command, Stdio},
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
pub struct Call {
    child: Child,
    done: bool,
    output: Option<std::thread::JoinHandle<std::io::Result<Vec<u8>>>>,
    pub input_bytes: usize,
    pub output_bytes: usize,
}
impl Call {
    pub fn start(name: &str, payload: &Value) -> Result<Self> {
        let bytes = serde_json::to_vec(payload)?;
        ensure!(bytes.len() <= CALL_INPUT_LIMIT, "oversized fixture Call");
        let mut child = Command::new(std::env::current_exe()?)
            .args(["call", name])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        // Read while the SDK process is running; a full pipe cannot let the
        // process exit before its original workload readback has been drained.
        let stdout = child.stdout.take().context("missing guest result")?;
        let output = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout
                .take((CALL_OUTPUT_LIMIT + 1) as u64)
                .read_to_end(&mut bytes)?;
            Ok(bytes)
        });
        let mut call = Self {
            child,
            done: false,
            output: Some(output),
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
            ensure!(exit.success(), "original guest Call failed");
            if !self
                .output
                .as_ref()
                .context("missing result reader")?
                .is_finished()
            {
                return Ok(None);
            }
            self.done = true;
            let bytes = self
                .output
                .take()
                .context("missing result reader")?
                .join()
                .map_err(|_| anyhow::anyhow!("guest result reader panicked"))??;
            ensure!(bytes.len() <= CALL_OUTPUT_LIMIT, "oversized guest result");
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
