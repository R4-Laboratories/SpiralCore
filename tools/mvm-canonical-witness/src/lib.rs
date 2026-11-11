#![forbid(unsafe_code)]
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::path::PathBuf;
mod payload;
pub use payload::expected_body;
#[cfg(all(feature = "native", target_os = "linux"))]
pub mod native;

#[derive(Debug)]
pub struct Config {
    pub home: PathBuf,
    pub rootfs: PathBuf,
    pub name: String,
    pub mailbox_hex: String,
    pub deadline_seconds: u64,
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.home.is_absolute() && self.rootfs.is_absolute(),
            "absolute home and rootfs required"
        );
        ensure!(
            self.name.starts_with("spiralcore-")
                && self.name.len() <= 63
                && self
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-'),
            "bounded VM name required"
        );
        ensure!(
            self.mailbox_hex.len() == 64 && self.mailbox_hex.bytes().all(|b| b.is_ascii_hexdigit()),
            "mailbox must contain 64 hex characters"
        );
        ensure!(
            (1..=300).contains(&self.deadline_seconds),
            "receive deadline must be 1 through 300 seconds"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ProcessSample {
    pub state: char,
    pub start_ticks: u64,
    pub user_ticks: u64,
    pub system_ticks: u64,
}
impl ProcessSample {
    pub fn parse(pid: u32, raw: &str) -> Result<Self> {
        let (prefix, fields) = raw
            .rsplit_once(") ")
            .context("invalid proc stat delimiter")?;
        let (recorded, _) = prefix.split_once(" (").context("invalid proc stat pid")?;
        ensure!(recorded.parse::<u32>()? == pid, "proc stat pid mismatch");
        let fields: Vec<_> = fields.split_whitespace().collect();
        let number = |index: usize| -> Result<u64> {
            fields
                .get(index)
                .context("short proc stat")?
                .parse()
                .context("invalid proc stat number")
        };
        let state = fields.first().context("missing process state")?;
        ensure!(state.len() == 1, "invalid process state");
        Ok(Self {
            state: state.chars().next().unwrap(),
            start_ticks: number(19)?,
            user_ticks: number(11)?,
            system_ticks: number(12)?,
        })
    }
}

#[derive(Debug)]
pub struct CleanupResult {
    pub stop: &'static str,
    pub cleanup: &'static str,
    pub error: Option<String>,
}
/// The actual native caller supplies its retained successful LaunchOutcome.
/// The generic parameter permits pure policy tests without constructing SDK authority.
pub async fn cleanup_after_launch<T>(
    original: Option<&T>,
    stop: impl std::future::Future<Output = Result<()>>,
    cleanup: impl FnOnce() -> Result<()>,
) -> CleanupResult {
    if original.is_none() {
        return CleanupResult {
            stop: "NOT_AUTHORIZED",
            cleanup: "RETAINED_UNOWNED_LAUNCH",
            error: None,
        };
    }
    if let Err(error) = stop.await {
        return CleanupResult {
            stop: "canonical_stop_failed",
            cleanup: "RETAINED_AFTER_STOP_REFUSAL",
            error: Some(format!("{error:#}")),
        };
    }
    match cleanup() {
        Ok(()) => CleanupResult {
            stop: "canonical_stop_ok",
            cleanup: "canonical_cleanup_ok",
            error: None,
        },
        Err(error) => CleanupResult {
            stop: "canonical_stop_ok",
            cleanup: "canonical_cleanup_failed",
            error: Some(format!("{error:#}")),
        },
    }
}
pub fn preflight_failure(error: &str) -> serde_json::Value {
    serde_json::json!({ "sdk_source": "ee632fbb3a746709c496a95f167559a01a19fcaf", "status": "failed_preflight", "native_transfer": "NOT_RUN", "received_bytes": null, "received_sha256": null, "error": error })
}
