#![forbid(unsafe_code)]
#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use spiralcore_mvm_canonical_witness::{Config, native, preflight_failure};
    use std::{fs, io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf};
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    anyhow::ensure!(
        args.len() == 4,
        "usage: spiralcore-mvm-canonical-witness ROOTFS NAME MAILBOX_HEX REPORT_JSON"
    );
    let output_path = PathBuf::from(&args[3]);
    let output_parent = fs::canonicalize(
        output_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("report parent required"))?,
    )?;
    let home = std::env::var_os("MVM_HOME").map(PathBuf::from);
    if let Some(home) = &home {
        anyhow::ensure!(
            !output_parent.starts_with(home)
                && !fs::canonicalize(home).is_ok_and(|root| output_parent.starts_with(root)),
            "report must remain outside MVM_HOME"
        );
    }
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&output_path)?;
    let result = (|| -> anyhow::Result<serde_json::Value> {
        let config = Config {
            home: home.ok_or_else(|| anyhow::anyhow!("MVM_HOME required"))?,
            rootfs: PathBuf::from(&args[0]),
            name: args[1]
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("UTF-8 name required"))?
                .into(),
            mailbox_hex: args[2]
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("UTF-8 mailbox required"))?
                .into(),
            deadline_seconds: 60,
        };
        config.validate()?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        Ok(serde_json::to_value(
            runtime.block_on(native::run(&config))?,
        )?)
    })();
    let record = match result {
        Ok(record) => record,
        Err(error) => preflight_failure(&format!("{error:#}")),
    };
    serde_json::to_writer_pretty(&mut output, &record)?;
    output.write_all(b"\n")?;
    output.sync_all()?;
    anyhow::ensure!(
        record["status"] == "passed",
        "witness failed; inspect retained JSON evidence"
    );
    Ok(())
}
#[cfg(not(target_os = "linux"))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("this native witness requires Linux/Firecracker")
}
