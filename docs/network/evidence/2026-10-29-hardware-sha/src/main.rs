use spiralcore_receiver_probe::{prepare, run_block, SOURCE_REVISION, SOURCE_SHA256};
use std::{collections::BTreeMap, path::Path};
fn main() {
    if let Err(e) = run() {
        eprintln!("receiver-probe: {e}");
        std::process::exit(1);
    }
}
fn run() -> spiralcore_receiver_probe::Result<()> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or("prepare or run required")?;
    let allowed = match command.as_str() {
        "prepare" => vec!["--root"],
        "run" => vec!["--root", "--out", "--block", "--variant"],
        _ => return Err("unknown command".into()),
    };
    let mut opts = BTreeMap::new();
    while let Some(key) = args.next() {
        if !allowed.contains(&key.as_str()) {
            return Err("unknown option".into());
        }
        let value = args.next().ok_or("option value required")?;
        if opts.insert(key, value).is_some() {
            return Err("duplicate option".into());
        }
    }
    let get = |name: &str| {
        opts.get(name)
            .map(String::as_str)
            .ok_or_else(|| format!("missing {name}"))
    };
    if SOURCE_REVISION == "unrecorded" || SOURCE_SHA256 == "unrecorded" {
        return Err(
            "compile with source revision and fingerprint before running diagnostics".into(),
        );
    }
    match command.as_str() {
        "prepare" => prepare(Path::new(get("--root")?)),
        "run" => run_block(
            Path::new(get("--root")?),
            Path::new(get("--out")?),
            get("--variant")?,
            get("--block")?.parse()?,
        ),
        _ => unreachable!(),
    }
}
