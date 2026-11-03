#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "linux", feature = "linux-host")
))]
fn main() {
    if let Err(error) = host_main() {
        eprintln!("spiralcore-mvm-live: {error}");
        std::process::exit(1);
    }
}
#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "linux", feature = "linux-host")
))]
fn host_main() -> spiralcore_mvm_live::Result<()> {
    use spiralcore_mvm_live::{
        config::{Plan, Stage, parse_arm, parse_bandwidth_profile},
        host,
        installed_owner::{InstallationKey, InstallationOwner, Recovery},
        installed_supervisor::{InstalledConfig, InstalledSupervisor},
        orchestrator,
    };
    use std::path::Path;
    let args: Vec<String> = std::env::args().collect();
    // This is a nonsecret local installation identifier, not a mailbox key.
    let installation_key = |id: &str| -> spiralcore_mvm_live::Result<InstallationKey> {
        if id.len() != 64 || !id.is_ascii() {
            return Err("installation ID must be 64 hex characters".into());
        }
        let mut installation = [0_u8; 32];
        for (index, byte) in installation.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&id[2 * index..2 * index + 2], 16)?;
        }
        Ok(InstallationKey {
            installation,
            profile: InstalledConfig::native_profile().profile_hash(),
        })
    };
    match args.get(1).map(String::as_str) {
        Some("installation-init") if args.len() == 4 => {
            let owner = InstallationOwner::initialize(Path::new(&args[2]), installation_key(&args[3])?)?;
            drop(owner);
            println!("{}", serde_json::json!({"ok":true,"installation_id":args[3],"phase":"clean","generation":0}));
        },
        Some("installation-reconcile") if args.len() == 4 => {
            let mut owner = InstallationOwner::open(Path::new(&args[2]), installation_key(&args[3])?)?;
            let retired = match owner.reconcile()? {
                Recovery::AlreadyClean => None,
                Recovery::Retired { epoch } => Some(epoch),
            };
            println!("{}", serde_json::json!({"ok":true,"phase":"clean","retired_generation":retired}));
        },
        Some("session-installed") if matches!(args.len(), 11 | 12) => {
            let stage = match args[7].as_str() { "one" => Stage::One, "two" => Stage::Two, _ => return Err("stage must be one or two".into()) };
            let plan = Plan { stage, arm: parse_arm(&args[8])?, size: args[9].parse()?, trial: args[10].parse()?, bandwidth_profile: parse_bandwidth_profile(args.get(11).map(String::as_str))? };
            let owner = InstallationOwner::open(Path::new(&args[2]), installation_key(&args[3])?)?;
            let mut controller = InstalledSupervisor::start(owner, InstalledConfig::native_profile())?;
            let result = orchestrator::session_installed(&mut controller, Path::new(&args[4]), Path::new(&args[5]), Path::new(&args[6]), plan);
            let retirement = controller.try_finish();
            match (result, retirement) {
                (Ok(value), Ok(())) => println!("{}", serde_json::json!({"ok":true,"result":format!("{}/result.json",args[4]),"bytes":value["logical_envelope_and_framing_bytes"],"installation_phase":"clean"})),
                (Err(error), Ok(())) => return Err(error),
                (Ok(_), Err(error)) => return Err(error.into()),
                (Err(error), Err(retirement)) => return Err(format!("{error}; installation retirement unresolved: {retirement}").into()),
            }
        },
        #[cfg(all(target_os = "linux", feature = "linux-host"))]
        Some("cleanup-worker") if args.len() == 3 => host::cleanup_worker(args[2].parse()?)?,
        Some("worker") if matches!(args.len(), 5 | 6)=>{
            let optional=|s:&str| if s=="-" {None} else {Some(Path::new(s).to_path_buf())};
            let disk=optional(&args[3]); let socket=optional(&args[4]);
            let profile=parse_bandwidth_profile(args.get(5).map(String::as_str))?;
            let value=host::worker_with_profile(Path::new(&args[2]),disk.as_deref(),socket.as_deref(),profile)?;
            println!("{}",serde_json::to_string(&value)?);
        },
        Some("worker-installed") if args.len() == 7 => {
            let optional = |s: &str| if s == "-" { None } else { Some(Path::new(s).to_path_buf()) };
            let disk = optional(&args[3]);
            let socket = optional(&args[4]);
            let profile = parse_bandwidth_profile(Some(&args[5]))?;
            let role: u8 = args[6].parse()?;
            if !(1..=3).contains(&role) { return Err("invalid installed worker role".into()); }
            let value = host::worker_installed(Path::new(&args[2]), disk.as_deref(), socket.as_deref(), profile, role)?;
            println!("{}", serde_json::to_string(&value)?);
        },
        Some("session") if matches!(args.len(), 9 | 10)=>{
            let stage=match args[5].as_str(){"one"=>Stage::One,"two"=>Stage::Two,_=>return Err("stage must be one or two".into())};
            let arm=parse_arm(&args[6])?;
            let bandwidth_profile=parse_bandwidth_profile(args.get(9).map(String::as_str))?;
            let result=orchestrator::session(Path::new(&args[2]),Path::new(&args[3]),Path::new(&args[4]),Plan{stage,arm,size:args[7].parse()?,trial:args[8].parse()?,bandwidth_profile})?;
            println!("{}",serde_json::json!({"ok":true,"result":format!("{}/result.json",args[2]),"bytes":result["logical_envelope_and_framing_bytes"]}));
        },
        _=>return Err("usage: spiralcore-mvm-live installation-init ROOT ID_HEX64 | installation-reconcile ROOT ID_HEX64 | session-installed ROOT ID_HEX64 NEW_DIR KERNEL GUEST_ELF one|two ARM SIZE TRIAL [standard|experimental64] | session NEW_DIR KERNEL GUEST_ELF one|two ARM SIZE TRIAL [standard|experimental64]; ARM=warm|whole|raw|combined|combined-cpu|whole-mixed".into()),
    }
    Ok(())
}
#[cfg(all(target_os = "linux", not(feature = "linux-host")))]
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
