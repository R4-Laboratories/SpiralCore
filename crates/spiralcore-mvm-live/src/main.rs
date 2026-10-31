#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn main() {
    if let Err(error) = host_main() {
        eprintln!("spiralcore-mvm-live: {error}");
        std::process::exit(1);
    }
}
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn host_main() -> spiralcore_mvm_live::Result<()> {
    use spiralcore_mvm_live::{
        config::{Arm, Plan, Stage},
        host, orchestrator,
    };
    use std::path::Path;
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("worker") if args.len()==5=>{
            let optional=|s:&str| if s=="-" {None} else {Some(Path::new(s).to_path_buf())};
            let disk=optional(&args[3]); let socket=optional(&args[4]);
            let value=host::worker(Path::new(&args[2]),disk.as_deref(),socket.as_deref())?;
            println!("{}",serde_json::to_string(&value)?);
        },
        Some("session") if args.len()==9=>{
            let stage=match args[5].as_str(){"one"=>Stage::One,"two"=>Stage::Two,_=>return Err("stage must be one or two".into())};
            let arm=match args[6].as_str(){"warm"=>Arm::Warm,"whole"=>Arm::Whole,"raw"=>Arm::Raw,_=>return Err("arm must be warm, whole or raw".into())};
            let result=orchestrator::session(Path::new(&args[2]),Path::new(&args[3]),Path::new(&args[4]),Plan{stage,arm,size:args[7].parse()?,trial:args[8].parse()?})?;
            println!("{}",serde_json::json!({"ok":true,"result":format!("{}/result.json",args[2]),"bytes":result["logical_envelope_and_framing_bytes"]}));
        },
        _=>return Err("usage: spiralcore-mvm-live session NEW_DIR KERNEL GUEST_ELF one|two warm|whole|raw SIZE TRIAL".into()),
    }
    Ok(())
}
#[cfg(target_os = "linux")]
fn main() {
    if spiralcore_mvm_live::guest::run().is_err() {
        eprintln!("guest shutdown or invocation failed");
        std::process::exit(1);
    }
}
#[cfg(not(any(target_os = "linux", all(target_os = "macos", target_arch = "aarch64"))))]
fn main() {
    eprintln!("This bounded witness requires Linux guests and an ARM64 macOS HVF host.");
    std::process::exit(1);
}
