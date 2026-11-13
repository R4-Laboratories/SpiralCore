use super::*;
use crate::installed_owner::{InstallationOwner, Phase};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::{
        fd::{AsRawFd, OwnedFd},
        unix::{fs::DirBuilderExt, process::CommandExt},
    },
    path::PathBuf,
    process::{Child, ChildStdout, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

const CHILD_MODE: &str = "SPIRAL_INSTALLED_WORKER_CHANNEL_CHILD";
const INPUT: &[u8] = b"synthetic initramfs fixture";
struct Case {
    base: PathBuf,
    binding: WorkerBinding,
    owner: InstallationOwner,
    epoch: crate::installed_owner::Epoch,
}
impl Case {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "spiral-worker-channel-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        let root = base.join("installation");
        let profile = crate::installed_supervisor::InstalledConfig::native_profile().profile_hash();
        let installation = [0x83; 32];
        let mut owner = InstallationOwner::initialize(
            &root,
            InstallationKey {
                installation,
                profile,
            },
        )
        .unwrap();
        let epoch = owner.begin().unwrap();
        Self {
            base,
            binding: WorkerBinding {
                root,
                installation,
                profile,
                generation: epoch.generation(),
                role: 1,
                slot: 0,
                serial: 1,
            },
            owner,
            epoch,
        }
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}
struct Target {
    child: Child,
    output: Option<BufReader<ChildStdout>>,
}
impl Target {
    fn spawn(endpoint: UnixStream, stdin: UnixStream, mode: &str) -> Self {
        let exact = format!(
            "{}::child_entry",
            module_path!().split_once("::").unwrap().1
        );
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", &exact, "--nocapture", "--test-threads=1"])
            .env(CHILD_MODE, mode)
            .stdin(Stdio::from(OwnedFd::from(stdin)))
            .stdout(if mode == "reject-death" {
                Stdio::null()
            } else {
                Stdio::piped()
            })
            .stderr(Stdio::inherit());
        let fd = endpoint.as_raw_fd();
        // SAFETY: only descriptor syscalls occur between fork/exec. The parent
        // retains endpoint until spawn returns; child owns only fixed fd198.
        unsafe {
            command.pre_exec(move || {
                if libc::dup2(fd, 198) < 0 || libc::fcntl(198, libc::F_SETFD, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().unwrap();
        drop(endpoint);
        let output = child.stdout.take().map(BufReader::new);
        Self { child, output }
    }
    fn event(&mut self, expected: &str) {
        loop {
            let mut line = String::new();
            assert!(self.output.as_mut().unwrap().read_line(&mut line).unwrap() > 0);
            if line.contains("EVENT ") {
                assert!(
                    line.trim_end().ends_with(expected),
                    "unexpected fixture event"
                );
                break;
            }
        }
    }
    fn finish(&mut self) {
        assert!(self.child.wait().unwrap().success());
    }
}
impl Drop for Target {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn five_records_authorize_exact_input_and_retain_actual_holder_until_context_exit() {
    let mut case = Case::new();
    let (mut parent, endpoint) = ParentControl::pair().unwrap();
    let (stdin, mut input) = UnixStream::pair().unwrap();
    let mut target = Target::spawn(endpoint, stdin, "success");
    parent.receive_challenge().unwrap();
    parent.send_binding(&case.binding).unwrap();
    parent.wait_ready().unwrap();
    parent.write_initrd(&mut input, INPUT).unwrap();
    parent.receive_boot_challenge().unwrap();
    parent.send_go(&case.binding).unwrap();
    target.event("AUTHORIZED");
    assert!(case.owner.finish(case.epoch).is_err());
    assert_eq!(case.owner.snapshot().unwrap().phase, Phase::Active);
    drop(parent);
    target.finish();
    case.owner.finish(case.epoch).unwrap();
}

#[test]
fn go_rejects_changed_original_facts_and_child_cannot_boot_after_refusal() {
    for change in 0..8 {
        let case = Case::new();
        let (mut parent, endpoint) = ParentControl::pair().unwrap();
        let (stdin, mut input) = UnixStream::pair().unwrap();
        let mut target = Target::spawn(endpoint, stdin, "reject");
        parent.receive_challenge().unwrap();
        parent.send_binding(&case.binding).unwrap();
        parent.wait_ready().unwrap();
        parent.write_initrd(&mut input, INPUT).unwrap();
        parent.receive_boot_challenge().unwrap();
        let mut changed = case.binding.clone();
        match change {
            0 => changed.root.push("other"),
            1 => changed.installation[0] ^= 1,
            2 => changed.profile[0] ^= 1,
            3 => changed.generation += 1,
            4 => changed.role = 2,
            5 => changed.slot = 1,
            6 => changed.serial += 1,
            _ => changed.root = PathBuf::from(format!("/{}", changed.root.display())),
        }
        assert!(parent.send_go(&changed).is_err());
        target.finish();
    }
}

#[test]
fn invalid_profile_role_and_closed_parent_refuse_before_child_activation() {
    for change in 0..3 {
        let case = Case::new();
        let (mut parent, endpoint) = ParentControl::pair().unwrap();
        let (stdin, _input) = UnixStream::pair().unwrap();
        let mut target = Target::spawn(endpoint, stdin, "reject");
        parent.receive_challenge().unwrap();
        if change == 2 {
            drop(parent);
        } else {
            let mut binding = case.binding.clone();
            if change == 0 {
                binding.profile[0] ^= 1;
            } else {
                binding.role = 0;
            }
            assert!(parent.send_binding(&binding).is_err());
        }
        target.finish();
    }
}

#[test]
fn oversize_and_partial_records_refuse_without_refreshing_original_deadline() {
    let (mut parent, mut endpoint) = ParentControl::pair().unwrap();
    endpoint.write_all(&1025_u32.to_be_bytes()).unwrap();
    assert!(parent.receive_challenge().is_err());
    let (mut parent, mut endpoint) = ParentControl::pair().unwrap();
    parent.deadline = Instant::now() + Duration::from_millis(10);
    endpoint.write_all(&[0, 0]).unwrap();
    assert!(matches!(
        parent.receive_challenge(),
        Err(ControlError::Deadline)
    ));
    assert!(parent.send_binding(&Case::new().binding).is_err());
}

#[test]
fn child_rejects_wrong_nonce_and_valid_but_wrong_expected_role() {
    for wrong_nonce in [true, false] {
        let case = Case::new();
        let (mut parent, endpoint) = ParentControl::pair().unwrap();
        let (stdin, _input) = UnixStream::pair().unwrap();
        let mut target = Target::spawn(endpoint, stdin, "reject");
        parent.receive_challenge().unwrap();
        let mut binding = case.binding.clone();
        let mut nonce = parent.first.unwrap();
        if wrong_nonce {
            nonce[0] ^= 1;
        } else {
            binding.role = 2;
        }
        write_record(
            &mut parent.stream,
            parent.deadline,
            &Record::Binding(nonce, binding),
        )
        .unwrap();
        target.finish();
    }
}

#[test]
fn child_rejects_a_forged_go_nonce_epoch_role_or_root_spelling() {
    for change in 0..4 {
        let case = Case::new();
        let (mut parent, endpoint) = ParentControl::pair().unwrap();
        let (stdin, mut input) = UnixStream::pair().unwrap();
        let mut target = Target::spawn(endpoint, stdin, "reject");
        parent.receive_challenge().unwrap();
        parent.send_binding(&case.binding).unwrap();
        parent.wait_ready().unwrap();
        parent.write_initrd(&mut input, INPUT).unwrap();
        parent.receive_boot_challenge().unwrap();
        let mut nonce = parent.second.unwrap();
        let mut binding = case.binding.clone();
        match change {
            0 => nonce[0] ^= 1,
            1 => binding.generation += 1,
            2 => binding.role = 2,
            _ => binding.root = PathBuf::from(format!("/{}", binding.root.display())),
        }
        write_record(
            &mut parent.stream,
            parent.deadline,
            &Record::Go(nonce, binding),
        )
        .unwrap();
        target.finish();
    }
}

#[test]
fn input_cap_and_empty_input_refuse_on_actual_local_stream() {
    for bytes in [b"".as_slice(), b"abcd".as_slice()] {
        let (mut writer, mut reader) = UnixStream::pair().unwrap();
        reader.set_nonblocking(true).unwrap();
        writer.write_all(bytes).unwrap();
        writer.shutdown(Shutdown::Write).unwrap();
        assert!(
            io_util::read_input(&mut reader, 3, Instant::now() + Duration::from_secs(1)).is_err()
        );
    }
}

#[test]
fn bounded_write_deadline_keeps_partial_progress_without_restart() {
    let (mut writer, mut reader) = UnixStream::pair().unwrap();
    writer.set_nonblocking(true).unwrap();
    reader.set_nonblocking(true).unwrap();
    let input = vec![0x39; 2 * 1024 * 1024];
    let deadline = Instant::now() + Duration::from_millis(10);
    assert!(matches!(
        io_util::write_all(&mut writer, &input, deadline),
        Err(ControlError::Deadline)
    ));
    let mut seen = [0; 32];
    assert!(
        reader.read(&mut seen).unwrap() > 0,
        "fixture must exercise a partial invoked write"
    );
    assert!(matches!(
        io_util::write_all(&mut writer, &[0], deadline),
        Err(ControlError::Deadline)
    ));
}

#[test]
fn actual_controller_death_before_go_refuses_and_worker_lifetime_exits() {
    mvm_core::util::test_env::isolated_lock_probe(probe_pre_go_controller_exit);
}

fn probe_pre_go_controller_exit() {
    let mut case = Case::new();
    let exact = format!(
        "{}::child_entry",
        module_path!().split_once("::").unwrap().1
    );
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", &exact, "--nocapture", "--test-threads=1"])
        .env(CHILD_MODE, "controller-death")
        .env("SPIRAL_CHANNEL_FIXTURE_ROOT", &case.binding.root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let mut child = command.spawn().unwrap();
    let mut controller = Target {
        output: Some(BufReader::new(child.stdout.take().unwrap())),
        child,
    };
    let worker_pid = loop {
        let mut line = String::new();
        assert!(
            controller
                .output
                .as_mut()
                .unwrap()
                .read_line(&mut line)
                .unwrap()
                > 0
        );
        if let Some((_, pid)) = line.split_once("EVENT WAITING_FOR_GO ") {
            break pid.trim().parse::<libc::pid_t>().unwrap();
        }
    };
    // Observe the original fixture worker before its owned controller dies.
    // This observer is not used for PID-based signaling or external reaping.
    let worker_exit = ProcessExitObserver::arm(worker_pid).unwrap();
    controller.child.kill().unwrap();
    assert!(!controller.child.wait().unwrap().success());
    assert!(matches!(
        worker_exit
            .wait_event(Instant::now() + Duration::from_secs(5))
            .unwrap(),
        ProcessExitWait::Exited
    ));
    assert_eq!(
        fs::read(case.binding.root.join("worker-refused-after-death")).unwrap(),
        b"refused"
    );
    case.owner.finish(case.epoch).unwrap();
}

#[test]
fn child_entry() {
    let Some(mode) = std::env::var_os(CHILD_MODE) else {
        return;
    };
    if mode == "controller-death" {
        let binding = WorkerBinding {
            root: std::env::var_os("SPIRAL_CHANNEL_FIXTURE_ROOT")
                .unwrap()
                .into(),
            installation: [0x83; 32],
            profile: crate::installed_supervisor::InstalledConfig::native_profile().profile_hash(),
            generation: 1,
            role: 1,
            slot: 0,
            serial: 1,
        };
        let (mut parent, endpoint) = ParentControl::pair().unwrap();
        let (stdin, mut input) = UnixStream::pair().unwrap();
        let target = Target::spawn(endpoint, stdin, "reject-death");
        parent.receive_challenge().unwrap();
        parent.send_binding(&binding).unwrap();
        parent.wait_ready().unwrap();
        parent.write_initrd(&mut input, INPUT).unwrap();
        println!("EVENT WAITING_FOR_GO {}", target.child.id());
        std::io::stdout().flush().unwrap();
        let mut command = String::new();
        // Root kills this exact owned fixture controller. EOF/unwind fallback
        // drops Target and reaps its owned child if the assertion path fails.
        let _ = std::io::stdin().read_line(&mut command);
        return;
    }
    let result = (|| -> Result<(), ControlError> {
        let mut context = InstalledWorkerContext::from_inherited_fd(1)?;
        let input = context.read_initrd()?;
        assert_eq!(input, INPUT);
        context.authorize_boot()?;
        let exported = context.holder_handoff()?;
        assert_eq!(exported.state_bytes, 176);
        drop(exported);
        println!("EVENT AUTHORIZED");
        std::io::stdout().flush().unwrap();
        context.stream.set_nonblocking(false).unwrap();
        context
            .stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut byte = [0];
        assert_eq!(context.stream.read(&mut byte).unwrap(), 0);
        Ok(())
    })();
    if mode == "reject" || mode == "reject-death" {
        assert!(result.is_err(), "rejected invocation authorized a backend");
        if mode == "reject-death" {
            let root = PathBuf::from(std::env::var_os("SPIRAL_CHANNEL_FIXTURE_ROOT").unwrap());
            fs::write(root.join("worker-refused-after-death"), b"refused").unwrap();
        }
    } else {
        result.unwrap();
    }
}
