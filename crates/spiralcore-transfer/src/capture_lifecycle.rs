//! Shared, bounded control-plane evidence for the experiment capture utility.
//! Quiet counters describe an observed interval, never the end of all network traffic.
use serde::{Deserialize, Serialize};

pub const SCHEMA: &str = "spiralcore.capture-lifecycle.v1";
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Counters {
    pub captured: u64,
    pub received: u64,
    pub dropped: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub elapsed_millis: u64,
    pub stderr_end: u64,
    pub file_bytes: u64,
    pub counters: Counters,
    pub stderr_text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lifecycle {
    pub schema: String,
    pub scope: String,
    pub interface: String,
    pub local_ip: String,
    pub remote_ip: String,
    pub port: u16,
    pub capture_pid: Option<u32>,
    pub tool_versions: String,
    pub tcpdump_sha256: String,
    pub pcap_sha256: Option<String>,
    pub stderr_sha256: Option<String>,
    pub flags: Vec<String>,
    pub ready: bool,
    pub ready_elapsed_millis: Option<u64>,
    pub preflight: Vec<Snapshot>,
    pub client_started_elapsed_millis: Option<u64>,
    pub client_ended_elapsed_millis: Option<u64>,
    pub client_exit_code: Option<i32>,
    pub quiescence: Vec<Snapshot>,
    pub interrupt_elapsed_millis: Option<u64>,
    pub capture_exit_code: Option<i32>,
    pub forced_cleanup: bool,
    pub final_snapshot: Option<Snapshot>,
    pub errors: Vec<String>,
}
impl Lifecycle {
    pub fn new(interface: String, local_ip: String, remote_ip: String, port: u16) -> Self {
        Self {
            schema: SCHEMA.into(),
            scope: "selected_capture_interval_only".into(),
            interface,
            local_ip,
            remote_ip,
            port,
            capture_pid: None,
            tool_versions: String::new(),
            tcpdump_sha256: String::new(),
            pcap_sha256: None,
            stderr_sha256: None,
            flags: Vec::new(),
            ready: false,
            ready_elapsed_millis: None,
            preflight: Vec::new(),
            client_started_elapsed_millis: None,
            client_ended_elapsed_millis: None,
            client_exit_code: None,
            quiescence: Vec::new(),
            interrupt_elapsed_millis: None,
            capture_exit_code: None,
            forced_cleanup: false,
            final_snapshot: None,
            errors: Vec::new(),
        }
    }
    pub fn failure_reasons(
        &self,
        final_counters: Option<Counters>,
        file_bytes: u64,
    ) -> Vec<String> {
        let mut failures = Vec::new();
        let mut check = |ok: bool, reason: &str| {
            if !ok {
                failures.push(reason.to_string());
            }
        };
        check(
            self.schema == SCHEMA && self.scope == "selected_capture_interval_only",
            "invalid_lifecycle_schema_or_scope",
        );
        check(
            self.capture_pid.is_some_and(|p| p > 1) && !self.interface.is_empty() && self.port > 0,
            "missing_capture_identity",
        );
        check(
            !self.tool_versions.is_empty()
                && self.tool_versions.len() <= 65536
                && hex_digest(&self.tcpdump_sha256),
            "missing_tool_provenance",
        );
        check(
            self.pcap_sha256.as_deref().is_some_and(hex_digest)
                && self.stderr_sha256.as_deref().is_some_and(hex_digest),
            "missing_final_integrity",
        );
        check(
            self.flags.iter().any(|s| s == "--immediate-mode")
                && self.flags.iter().any(|s| s == "-U"),
            "missing_capture_flags",
        );
        check(self.errors.is_empty(), "controller_reported_failure");
        check(
            self.ready && self.ready_elapsed_millis.is_some_and(|t| t <= 10000),
            "capture_not_ready",
        );
        check(
            self.client_exit_code == Some(0),
            "client_failed_or_not_reaped",
        );
        check(
            self.capture_exit_code == Some(0) && !self.forced_cleanup,
            "capture_failed_or_not_cleanly_reaped",
        );
        check(
            self.preflight.len() == 2 && quiet(&self.preflight),
            "preflight_not_quiet_and_complete",
        );
        check(
            self.quiescence.len() >= 3
                && self.quiescence.len() <= 21
                && quiet(&self.quiescence[self.quiescence.len().saturating_sub(3)..]),
            "post_client_not_quiet_and_complete",
        );
        let ordered = || -> Option<bool> {
            let ready = self.ready_elapsed_millis?;
            let prefirst = self.preflight.first()?;
            let prelast = self.preflight.last()?;
            let start = self.client_started_elapsed_millis?;
            let end = self.client_ended_elapsed_millis?;
            let quietfirst = self.quiescence.first()?;
            let quietlast = self.quiescence.last()?;
            let interrupt = self.interrupt_elapsed_millis?;
            let final_snapshot = self.final_snapshot.as_ref()?;
            Some(
                ready <= prefirst.elapsed_millis
                    && prelast.elapsed_millis <= start
                    && start <= end
                    && end <= quietfirst.elapsed_millis
                    && quietlast.elapsed_millis.saturating_sub(end) <= 10000
                    && quietlast.elapsed_millis <= interrupt
                    && interrupt <= final_snapshot.elapsed_millis
                    && final_snapshot.elapsed_millis.saturating_sub(interrupt) <= 5000,
            )
        };
        check(
            ordered() == Some(true),
            "lifecycle_phase_order_or_deadline_failed",
        );
        let samples = self
            .preflight
            .iter()
            .chain(&self.quiescence)
            .chain(self.final_snapshot.iter())
            .collect::<Vec<_>>();
        check(
            samples.windows(2).all(|p| {
                p[0].elapsed_millis <= p[1].elapsed_millis
                    && p[1].stderr_end.checked_sub(p[0].stderr_end)
                        == Some(p[1].stderr_text.len() as u64)
                    && p[0].file_bytes <= p[1].file_bytes
                    && p[0].counters.captured <= p[1].counters.captured
                    && p[0].counters.received <= p[1].counters.received
                    && p[0].counters.dropped <= p[1].counters.dropped
            }),
            "lifecycle_counters_or_offsets_regressed",
        );
        check(
            samples.iter().all(|s| {
                s.stderr_text.len() <= 4096 && parse_counter_block(&s.stderr_text) == Ok(s.counters)
            }),
            "snapshot_text_counter_mismatch",
        );
        check(
            samples.iter().all(|s| {
                s.stderr_end <= 1024 * 1024
                    && (24..=256 * 1024 * 1024).contains(&s.file_bytes)
                    && s.counters.dropped == 0
            }),
            "lifecycle_drop_or_size_limit",
        );
        check(
            self.final_snapshot.as_ref().is_some_and(|f| {
                Some(f.counters) == final_counters
                    && f.file_bytes == file_bytes
                    && self
                        .quiescence
                        .last()
                        .is_some_and(|q| q.file_bytes == f.file_bytes && q.counters == f.counters)
            }),
            "final_stats_not_independent_or_changed_after_quiet",
        );
        failures
    }
}

pub fn hex_digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub fn quiet(s: &[Snapshot]) -> bool {
    s.len() >= 2
        && s.iter().all(|v| {
            v.counters == s[0].counters
                && v.file_bytes == s[0].file_bytes
                && v.counters.captured == v.counters.received
                && v.counters.dropped == 0
        })
        && s.windows(2)
            .all(|p| p[1].elapsed_millis.saturating_sub(p[0].elapsed_millis) >= 500)
}
pub fn parse_counter_block(text: &str) -> Result<Counters, String> {
    if text.len() > 1024 * 1024 || !text.ends_with('\n') {
        return Err("truncated or oversized counter block".into());
    }
    let mut values = Vec::new();
    for line in text.lines() {
        let words = line.split_whitespace().collect::<Vec<_>>();
        if words.len() < 3 || !matches!(words[1], "packet" | "packets") {
            continue;
        }
        let index = match &words[2..] {
            ["captured"] => 0,
            ["received", "by", "filter"] => 1,
            ["dropped", "by", "kernel"] => 2,
            _ => continue,
        };
        if index != values.len() {
            return Err("duplicate or out-of-order counter block".into());
        }
        let n = words[0]
            .parse::<u64>()
            .map_err(|_| "invalid counter integer")?;
        values.push(n);
    }
    if values.len() != 3 {
        return Err("incomplete counter block".into());
    }
    Ok(Counters {
        captured: values[0],
        received: values[1],
        dropped: values[2],
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    fn snap(at: u64, end: u64, count: u64) -> Snapshot {
        Snapshot {
            elapsed_millis: at,
            stderr_end: end,
            file_bytes: 24 + count * 112,
            counters: Counters {
                captured: count,
                received: count,
                dropped: 0,
            },
            stderr_text:format!("{count} packets captured\n{count} packets received by filter\n0 packets dropped by kernel\n"),
        }
    }
    pub(crate) fn valid() -> Lifecycle {
        let mut l = Lifecycle::new("eth0".into(), "10.0.0.1".into(), "10.0.0.2".into(), 443);
        l.capture_pid = Some(100);
        l.tool_versions = "tcpdump test; libpcap test".into();
        l.tcpdump_sha256 = "a".repeat(64);
        l.pcap_sha256 = Some("b".repeat(64));
        l.stderr_sha256 = Some("c".repeat(64));
        l.flags = vec![
            "--immediate-mode".into(),
            "-B".into(),
            "4096".into(),
            "-U".into(),
            "-s".into(),
            "96".into(),
        ];
        l.ready = true;
        l.ready_elapsed_millis = Some(100);
        l.preflight = vec![snap(200, 100, 0), snap(700, 200, 0)];
        l.client_started_elapsed_millis = Some(701);
        l.client_ended_elapsed_millis = Some(2000);
        l.client_exit_code = Some(0);
        l.quiescence = vec![
            snap(2100, 300, 10),
            snap(2600, 400, 10),
            snap(3100, 500, 10),
        ];
        l.interrupt_elapsed_millis = Some(3101);
        l.capture_exit_code = Some(0);
        l.final_snapshot = Some(snap(3150, 600, 10));
        let mut offset = 30;
        for sample in l
            .preflight
            .iter_mut()
            .chain(&mut l.quiescence)
            .chain(l.final_snapshot.iter_mut())
        {
            offset += sample.stderr_text.len() as u64;
            sample.stderr_end = offset;
        }
        l
    }
    #[test]
    fn qualified_control_evidence_requires_a_complete_ordered_lifecycle() {
        let l = valid();
        let f = l.final_snapshot.as_ref().unwrap();
        assert!(l.failure_reasons(Some(f.counters), f.file_bytes).is_empty());
        for change in 0..14 {
            let mut bad = valid();
            match change {
                0 => bad.ready = false,
                1 => bad.preflight[0].counters.received = 3,
                2 => bad.client_started_elapsed_millis = Some(50),
                3 => bad.quiescence[2].file_bytes += 1,
                4 => bad.capture_exit_code = Some(1),
                5 => bad.forced_cleanup = true,
                6 => bad.final_snapshot = None,
                7 => bad.client_exit_code = Some(1),
                8 => bad.ready_elapsed_millis = Some(10001),
                9 => bad.quiescence.truncate(2),
                10 => bad.errors.push("signal failed".into()),
                11 => bad.quiescence[0].counters.dropped = 1,
                12 => {
                    bad.final_snapshot.as_mut().unwrap().stderr_end =
                        bad.quiescence.last().unwrap().stderr_end
                }
                _ => {
                    bad.final_snapshot.as_mut().unwrap().stderr_text =
                        "10 packets captured\n".into()
                }
            }
            assert!(
                !bad.failure_reasons(Some(f.counters), f.file_bytes)
                    .is_empty(),
                "change {change}"
            );
        }
    }
    #[test]
    fn old_counter_gap_is_not_normalized_by_new_control_evidence() {
        let mut l = valid();
        l.final_snapshot.as_mut().unwrap().counters.received += 3;
        let f = l.final_snapshot.as_ref().unwrap();
        assert!(!l.failure_reasons(Some(f.counters), f.file_bytes).is_empty());
    }
    #[test]
    fn stats_samples_must_be_complete_separate_blocks() {
        let text =
            "10 packets captured\n10 packets received by filter\n0 packets dropped by kernel\n";
        assert_eq!(
            parse_counter_block(text).unwrap(),
            Counters {
                captured: 10,
                received: 10,
                dropped: 0
            }
        );
        assert!(parse_counter_block(&(text.to_owned() + text)).is_err());
        assert!(
            parse_counter_block("10 packets captured\n10 packets received by filter\n").is_err()
        );
        assert!(parse_counter_block("1 packets captured\n18446744073709551616 packets received by filter\n0 packets dropped by kernel\n").is_err());
    }
    #[test]
    fn changing_or_regressing_counters_are_not_a_quiet_interval() {
        let mut l = valid();
        let f = l.final_snapshot.as_ref().unwrap().clone();
        l.quiescence[0].counters.captured = 11;
        l.quiescence[0].counters.received = 11;
        assert!(!l.failure_reasons(Some(f.counters), f.file_bytes).is_empty());
        let mut l = valid();
        l.quiescence[1].elapsed_millis = 3099;
        assert!(!l.failure_reasons(Some(f.counters), f.file_bytes).is_empty());
    }
}
