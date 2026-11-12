#![forbid(unsafe_code)]
#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use spiralcore_mvm_canonical_witness::{Config, native, preflight_failure};
    use std::{fs, io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf};
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let paired = args.first().is_some_and(|arg| arg == "--pair");
    let addressed = args.first().is_some_and(|arg| arg == "--addressed");
    anyhow::ensure!(
        !addressed || cfg!(feature = "addressed"),
        "addressed terminal requires the addressed build"
    );
    let args = if paired || addressed {
        &args[1..]
    } else {
        &args[..]
    };
    anyhow::ensure!(
        args.len()
            == if addressed {
                7
            } else if paired {
                6
            } else {
                4
            },
        "usage: spiralcore-mvm-canonical-witness ROOTFS NAME MAILBOX_HEX REPORT_JSON; paired: --pair ROOTFS NAME_A MAILBOX_A NAME_B MAILBOX_B REPORT_JSON; --addressed ROOTFS_A NAME_A MAILBOX_A ROOTFS_B NAME_B MAILBOX_B REPORT_JSON"
    );
    let output_path = PathBuf::from(args.last().expect("checked parameters"));
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
    let mut reported = false;
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
        #[cfg(feature = "addressed")]
        if addressed {
            use std::io::Read;
            let mut b = config.clone();
            b.rootfs = PathBuf::from(&args[3]);
            b.name = args[4]
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("UTF-8 name required"))?
                .into();
            b.mailbox_hex = args[5]
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("UTF-8 mailbox required"))?
                .into();
            let mut body = [0; 257];
            fs::File::open("/dev/urandom")?.read_exact(&mut body)?;
            body[0] = 0;
            body[1] = 255;
            body[256] = 0;
            return native::addressed_pair::run(
                &config,
                &b,
                &body,
                |failure| {
                    reported = true;
                    serde_json::to_writer_pretty(&mut output, failure)?;
                    output.write_all(b"\n")?;
                    output.sync_all()?;
                    fs::File::open(&output_parent)?.sync_all()?;
                    eprintln!(
                        "addressed coordinator deadline recorded; original retirement pending"
                    );
                    Ok(())
                },
                |status| {
                    let mut path = output_path.as_os_str().to_os_string();
                    path.push(".coordinator-exit.json");
                    let mut late = fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .mode(0o600)
                        .open(path)?;
                    let observation = serde_json::json!({"origin":"retained original Child", "exit_status":status.to_string(), "exit_code":status.code()});
                    serde_json::to_writer(&mut late, &observation)?;
                    late.write_all(b"\n")?;
                    late.sync_all()?;
                    fs::File::open(&output_parent)?.sync_all()?;
                    Ok(())
                },
            );
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        if paired {
            let mut second = config.clone();
            second.name = args[3]
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("UTF-8 name required"))?
                .into();
            second.mailbox_hex = args[4]
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("UTF-8 mailbox required"))?
                .into();
            runtime.block_on(native::run_pair(&config, &second))
        } else {
            Ok(serde_json::to_value(
                runtime.block_on(native::run(&config))?,
            )?)
        }
    })();
    let record = match result {
        Ok(record) => record,
        Err(error) => preflight_failure(&format!("{error:#}")),
    };
    if !reported {
        serde_json::to_writer_pretty(&mut output, &record)?;
        output.write_all(b"\n")?;
        output.sync_all()?;
    }
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
