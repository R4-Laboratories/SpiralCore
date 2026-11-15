//! Observation through the original Child ID; /proc data never grants custody.
use super::*;
use std::fs;

#[derive(serde::Serialize)]
pub(super) struct Sample {
    pub pid: u32,
    pub virtual_bytes: u64,
    pub virtual_peak_bytes: u64,
    pub resident_bytes: u64,
    pub resident_peak_bytes: u64,
    pub threads: u64,
    pub open_fds: usize,
    pub original_store_open_fds: usize,
    pub start_ticks: u64,
    pub address_space_limit: u64,
}

pub(super) fn sample(pid: u32, store: &std::path::Path) -> Result<Sample> {
    let root = std::path::PathBuf::from(format!("/proc/{pid}"));
    let status = fs::read_to_string(root.join("status"))?;
    let value = |name: &str| -> Result<u64> {
        let line = status
            .lines()
            .find(|line| line.starts_with(name))
            .context("process field missing")?;
        let mut parts = line.split_whitespace();
        parts.next();
        let count: u64 = parts.next().context("process value missing")?.parse()?;
        Ok(match parts.next() {
            Some("kB") => count.checked_mul(1024).context("process bytes overflow")?,
            None => count,
            _ => anyhow::bail!("unknown process units"),
        })
    };
    let limits = fs::read_to_string(root.join("limits"))?;
    let line = limits
        .lines()
        .find(|line| line.starts_with("Max address space"))
        .context("RLIMIT_AS missing")?;
    let mut fields = line
        .trim_start_matches("Max address space")
        .split_whitespace();
    let soft: u64 = fields.next().context("soft RLIMIT_AS missing")?.parse()?;
    let hard: u64 = fields.next().context("hard RLIMIT_AS missing")?.parse()?;
    ensure!(
        soft == hard && fields.next() == Some("bytes"),
        "effective RLIMIT_AS differs"
    );
    let descriptors = fs::read_dir(root.join("fd"))?.collect::<std::io::Result<Vec<_>>>()?;
    let original_store_open_fds = descriptors
        .iter()
        .filter(|entry| fs::read_link(entry.path()).is_ok_and(|p| p.starts_with(store)))
        .count();
    let stat = fs::read_to_string(root.join("stat"))?;
    let fields = stat.rsplit_once(") ").context("process stat shape")?.1;
    let start_ticks = fields
        .split_whitespace()
        .nth(19)
        .context("process start time missing")?
        .parse()?;
    Ok(Sample {
        pid,
        virtual_bytes: value("VmSize:")?,
        virtual_peak_bytes: value("VmPeak:")?,
        resident_bytes: value("VmRSS:")?,
        resident_peak_bytes: value("VmHWM:")?,
        threads: value("Threads:")?,
        open_fds: descriptors.len(),
        original_store_open_fds,
        start_ticks,
        address_space_limit: soft,
    })
}

pub(super) fn direct_children() -> Result<std::collections::BTreeSet<u32>> {
    fs::read_to_string("/proc/thread-self/children")?
        .split_whitespace()
        .map(|pid| pid.parse().map_err(Into::into))
        .collect()
}

pub(super) fn children_cpu_ns() -> Result<u128> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // SAFETY: getrusage initializes this writable rusage on success.
    ensure!(
        unsafe { libc::getrusage(libc::RUSAGE_CHILDREN, usage.as_mut_ptr()) } == 0,
        "getrusage children: {}",
        std::io::Error::last_os_error()
    );
    // SAFETY: the successful syscall initialized every rusage field.
    let usage = unsafe { usage.assume_init() };
    let ns = |v: libc::timeval| -> Result<u128> {
        u128::try_from(v.tv_sec)?
            .checked_mul(1_000_000_000)
            .and_then(|s| s.checked_add(u128::try_from(v.tv_usec).ok()?.checked_mul(1000)?))
            .context("child CPU overflow")
    };
    ns(usage.ru_utime)?
        .checked_add(ns(usage.ru_stime)?)
        .context("child CPU sum overflow")
}
