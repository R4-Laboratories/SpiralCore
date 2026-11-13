//! Bounded observation of the original guest executable's two measurement rows.
use anyhow::{Result, ensure};
use mvm_client::entrypoint::{CallObserver, ShownChunk};
use mvm_contract::stream::StreamKind;
use serde::{Deserialize, Serialize};

const MAX_REPORT: usize = 4096;

#[derive(Default)]
pub(super) struct GuestOutput {
    bytes: Vec<u8>,
    overflow: bool,
    pub(super) stderr_bytes: usize,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Phase {
    phase: Name,
    guest_proxy: Counts,
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
enum Name {
    Denied,
    Echo,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Counts {
    completed: bool,
    connect_request_bytes: u64,
    connect_response_bytes: u64,
    application_sent_bytes: u64,
    application_received_bytes: u64,
    process_cpu_ns: u64,
    elapsed_ns: u64,
}

impl GuestOutput {
    fn stdout(&mut self, chunk: &[u8]) {
        if self.overflow || chunk.len() > MAX_REPORT - self.bytes.len() {
            self.overflow = true;
        } else {
            self.bytes.extend_from_slice(chunk);
        }
    }

    /// Completed JSON lines, including the original failed phase's actual counts.
    /// Unknown fields or phase names never reach the qualification log.
    pub(super) fn report(&self) -> Result<Vec<Phase>> {
        ensure!(!self.overflow, "guest stdout exceeded its bounded report");
        let mut phases = Vec::with_capacity(2);
        for line in std::str::from_utf8(&self.bytes)?.lines() {
            ensure!(phases.len() < 2, "additional guest stdout");
            phases.push(serde_json::from_str(line)?);
        }
        Ok(phases)
    }
}

pub(super) fn verify(phases: &[Phase]) -> Result<()> {
    let [deny, echo] = phases else {
        anyhow::bail!("missing guest phases")
    };
    for (row, name, head, body) in [
        (
            deny,
            Name::Denied,
            b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".len(),
            0,
        ),
        (
            echo,
            Name::Echo,
            b"HTTP/1.1 200 Connection established\r\n\r\n".len(),
            5804,
        ),
    ] {
        let count = &row.guest_proxy;
        ensure!(
            row.phase == name
                && count.completed
                && count.connect_request_bytes > 0
                && count.connect_response_bytes == head as u64
                && count.application_sent_bytes == body
                && count.application_received_bytes == body,
            "guest {name:?} counts or completion differed"
        );
    }
    Ok(())
}

impl CallObserver for GuestOutput {
    fn output(&mut self, chunk: &ShownChunk) {
        match chunk.kind {
            StreamKind::Stdout => self.stdout(&chunk.body),
            StreamKind::Stderr => {
                self.stderr_bytes = self.stderr_bytes.saturating_add(chunk.body.len())
            }
            _ => {}
        }
    }

    fn control(&mut self, _header: &str, _payload_len: usize) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    const DENY: &[u8] = br#"{"phase":"denied","guest_proxy":{"completed":true,"connect_request_bytes":1,"connect_response_bytes":64,"application_sent_bytes":0,"application_received_bytes":0,"process_cpu_ns":1,"elapsed_ns":2}}"#;
    const ECHO: &[u8] = br#"{"phase":"echo","guest_proxy":{"completed":true,"connect_request_bytes":1,"connect_response_bytes":39,"application_sent_bytes":5804,"application_received_bytes":5804,"process_cpu_ns":1,"elapsed_ns":2}}"#;

    fn capture(rows: &[&[u8]]) -> GuestOutput {
        let mut output = GuestOutput::default();
        for row in rows {
            for chunk in row.chunks(7) {
                output.stdout(chunk);
            }
            output.stdout(b"\n");
        }
        output
    }

    #[test]
    fn requires_both_complete_ordered_original_phase_reports() {
        assert!(verify(&capture(&[DENY, ECHO]).report().unwrap()).is_ok());
        for rows in [vec![DENY], vec![ECHO, DENY], vec![DENY, ECHO, ECHO]] {
            assert!(capture(&rows).report().and_then(|p| verify(&p)).is_err());
        }
        let short = String::from_utf8_lossy(ECHO).replace("5804", "5803");
        assert!(verify(&capture(&[DENY, short.as_bytes()]).report().unwrap()).is_err());
    }

    #[test]
    fn output_overflow_is_sticky_and_never_retains_the_extra_chunk() {
        let mut output = capture(&[DENY, ECHO]);
        output.stdout(&[0; 4097]);
        assert!(output.report().is_err());
        assert!(output.bytes.len() <= 4096);
    }
}
