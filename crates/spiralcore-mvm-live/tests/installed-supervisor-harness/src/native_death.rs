//! Explicit Linux/KVM acceptance runner, never invoked by ordinary unit tests.
//! Arguments: immutable host executable, kernel, static guest, NEW private root,
//! and expected Firecracker executable. Requires the separately admitted host.
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("native death acceptance requires Linux/KVM");
    std::process::exit(2);
}

#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    linux::run()
}

#[cfg(target_os = "linux")]
mod linux {
    use installed_supervisor_harness::{
        installed_owner::{InstallationKey, InstallationOwner, OwnerError, Phase, Recovery},
        installed_supervisor::InstalledConfig,
    };
    use std::{
        fs::{self, File},
        io,
        os::{
            fd::{AsRawFd, FromRawFd, OwnedFd},
            unix::{
                ffi::OsStrExt,
                fs::{MetadataExt, PermissionsExt},
            },
        },
        path::{Path, PathBuf},
        process::{Child, Command, Stdio},
        thread,
        time::{Duration, Instant},
    };

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
    const LIMIT: Duration = Duration::from_secs(40);

    fn require(value: bool, message: &'static str) -> Result<()> {
        if value {
            Ok(())
        } else {
            Err(io::Error::other(message).into())
        }
    }
    fn same_inode(a: &Path, b: &Path) -> io::Result<bool> {
        let a = fs::metadata(a)?;
        let b = fs::metadata(b)?;
        Ok(a.dev() == b.dev() && a.ino() == b.ino())
    }
    fn birth(pid: u32) -> io::Result<String> {
        let raw = fs::read_to_string(format!("/proc/{pid}/stat"))?;
        raw.rsplit_once(')')
            .and_then(|(_, rest)| rest.split_whitespace().nth(19))
            .map(str::to_owned)
            .ok_or_else(|| io::Error::other("invalid proc starttime"))
    }
    struct ExactProcess {
        fd: OwnedFd,
        pid: u32,
        kill_on_drop: bool,
    }
    impl ExactProcess {
        fn capture(pid: u32, validate: impl Fn() -> bool) -> Result<Self> {
            let started = birth(pid)?;
            require(validate(), "live process identity refused before pidfd")?;
            // SAFETY: scalar pidfd_open obtains an owned reference; it sends no signal.
            let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
            if raw < 0 {
                return Err(io::Error::last_os_error().into());
            }
            // SAFETY: successful pidfd_open returned a fresh descriptor.
            let fd = unsafe { OwnedFd::from_raw_fd(raw as i32) };
            require(
                birth(pid)? == started && validate(),
                "process changed while arming pidfd",
            )?;
            Ok(Self {
                fd,
                pid,
                kill_on_drop: false,
            })
        }
        fn signal(&self, signal: i32) -> Result<()> {
            // SAFETY: live owned pidfd and scalar signal; no numeric-PID fallback.
            let rc = unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    self.fd.as_raw_fd(),
                    signal,
                    std::ptr::null::<libc::siginfo_t>(),
                    0,
                )
            };
            if rc < 0 {
                return Err(io::Error::last_os_error().into());
            }
            Ok(())
        }
        fn exited(&self) -> io::Result<bool> {
            let mut poll = libc::pollfd {
                fd: self.fd.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: one initialized pollfd, no blocking timeout.
            let rc = unsafe { libc::poll(&mut poll, 1, 0) };
            if rc < 0 {
                return Err(io::Error::last_os_error());
            }
            if poll.revents & (libc::POLLERR | libc::POLLNVAL) != 0 {
                return Err(io::Error::other("pidfd observation failed"));
            }
            Ok(rc > 0 && poll.revents & libc::POLLIN != 0)
        }
        fn wait_exit(&self) -> Result<()> {
            until(
                || self.exited().unwrap_or(false),
                "exact process did not exit",
            )
        }
        fn stop(&self) -> Result<()> {
            self.signal(libc::SIGSTOP)?;
            until(
                || {
                    fs::read_to_string(format!("/proc/{}/status", self.pid)).is_ok_and(|s| {
                        s.lines()
                            .any(|l| l.starts_with("State:") && l.contains("T (stopped)"))
                    })
                },
                "exact process did not stop",
            )
        }
    }
    impl Drop for ExactProcess {
        fn drop(&mut self) {
            if !self.kill_on_drop {
                return;
            }
            if self.exited().unwrap_or(false) {
                return;
            }
            let _ = self.signal(libc::SIGKILL);
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline && !self.exited().unwrap_or(false) {
                thread::sleep(Duration::from_millis(5));
            }
            if !self.exited().unwrap_or(false) {
                eprintln!("HOLD: exact captured process exit remains unconfirmed");
            }
        }
    }
    struct ExactChild(Child);
    impl Drop for ExactChild {
        fn drop(&mut self) {
            // Before capturing the backend, killing this owner could orphan an
            // undiscovered VMM. Give its production 120s watchdog/cleanup time;
            // unknown completion never authorizes a fixture PID-based kill.
            let deadline = Instant::now() + Duration::from_secs(150);
            while Instant::now() < deadline {
                match self.0.try_wait() {
                    Ok(Some(_)) => return,
                    Ok(None) => thread::sleep(Duration::from_millis(5)),
                    Err(_) => break,
                }
            }
            eprintln!("HOLD: owned supervisor completion is unknown; leaving its custody intact");
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn pre_backend_discovery_drop_allows_owned_supervisor_to_finish() {
            let root =
                std::env::temp_dir().join(format!("sc-native-cleanup-{}", std::process::id()));
            fs::create_dir(&root).unwrap();
            let completed = root.join("completed");
            let child = Command::new("/bin/sh")
                .args(["-c", "sleep 0.03; printf clean > \"$1\"", "cleanup-test"])
                .arg(&completed)
                .spawn()
                .unwrap();
            drop(ExactChild(child));
            let observed = fs::read(&completed);
            fs::remove_dir_all(root).unwrap();
            assert_eq!(
                observed.unwrap(),
                b"clean",
                "unknown backend discovery must not kill its owner"
            );
        }
    }
    fn until(mut predicate: impl FnMut() -> bool, message: &'static str) -> Result<()> {
        let deadline = Instant::now() + LIMIT;
        while Instant::now() < deadline {
            if predicate() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(2));
        }
        Err(io::Error::other(message).into())
    }
    fn child_ids(parent: u32) -> io::Result<Vec<u32>> {
        fs::read_to_string(format!("/proc/{parent}/task/{parent}/children"))?
            .split_whitespace()
            .map(|p| p.parse().map_err(io::Error::other))
            .collect()
    }
    fn worker_matches(pid: u32, parent: u32, host: &Path, kernel: &Path, run: &Path) -> bool {
        let Ok(children) = child_ids(parent) else {
            return false;
        };
        if !children.contains(&pid)
            || !same_inode(Path::new(&format!("/proc/{pid}/exe")), host).unwrap_or(false)
        {
            return false;
        }
        let worker_uid = fs::metadata(format!("/proc/{pid}")).map(|m| m.uid());
        let parent_uid = fs::metadata(format!("/proc/{parent}")).map(|m| m.uid());
        if !matches!((worker_uid,parent_uid),(Ok(a),Ok(b)) if a == b && a != 0) {
            return false;
        }
        let Ok(args) = fs::read(format!("/proc/{pid}/cmdline")) else {
            return false;
        };
        let args = args
            .split(|b| *b == 0)
            .filter(|b| !b.is_empty())
            .collect::<Vec<_>>();
        if args.len() != 7
            || args[1] != b"worker-installed"
            || args[2] != kernel.as_os_str().as_bytes()
            || args[3] != b"-"
            || args[5] != b"standard"
            || args[6] != b"1"
        {
            return false;
        }
        let socket = Path::new(std::ffi::OsStr::from_bytes(args[4]));
        let expected_prefix = format!("scmvm-{parent}-");
        if socket.file_name() != Some(std::ffi::OsStr::new("a.sock"))
            || !socket
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|name| {
                    let bytes = name.as_bytes();
                    bytes.starts_with(expected_prefix.as_bytes())
                        && bytes.len() == expected_prefix.len() + 16
                        && bytes[expected_prefix.len()..]
                            .iter()
                            .all(u8::is_ascii_hexdigit)
                })
            || socket.parent().and_then(Path::parent) != Some(std::env::temp_dir().as_path())
        {
            return false;
        }
        let expected = format!("MVM_HOME={}", run.join("mvm-sender").display());
        fs::read(format!("/proc/{pid}/environ")).is_ok_and(|e| {
            e.split(|b| *b == 0)
                .any(|entry| entry == expected.as_bytes())
        })
    }
    fn backend_matches(pid: u32, fc: &Path, holders: &Path, run: &Path) -> bool {
        let process = PathBuf::from(format!("/proc/{pid}"));
        // Bounded fixture diagnostics: only the backend name, identity checks,
        // and descriptor counts. Never argv, environment or bootstrap bytes.
        static LAST_OBSERVED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let observed = fs::read_to_string(process.join("comm"))
            .is_ok_and(|s| s.trim() == "firecracker")
            && LAST_OBSERVED.swap(pid, std::sync::atomic::Ordering::Relaxed) != pid;
        if observed {
            eprintln!(
                "backend candidate pid={pid} executable_check={:?} uid={:?} fd_access={:?}",
                same_inode(&process.join("exe"), fc),
                fs::metadata(&process).map(|m| m.uid()),
                fs::read_dir(process.join("fd")).map(|d| d.count())
            );
        }
        if !same_inode(&process.join("exe"), fc).unwrap_or(false) {
            return false;
        }
        let Ok(meta) = fs::metadata(&process) else {
            return false;
        };
        // SAFETY: geteuid only reads the runner's identity.
        if meta.uid() != unsafe { libc::geteuid() } {
            return false;
        }
        let Ok(fds) = fs::read_dir(process.join("fd")) else {
            return false;
        };
        let mut holder = false;
        let mut console = false;
        for fd in fds.flatten() {
            holder |= same_inode(&fd.path(), holders).unwrap_or(false);
            console |= fs::read_link(fd.path()).is_ok_and(|p| {
                p.starts_with(run.join("mvm-sender"))
                    && p.file_name() == Some(std::ffi::OsStr::new("console.log"))
            });
        }
        if observed {
            eprintln!("backend candidate pid={pid} holder={holder} run_console={console}");
        }
        holder && console
    }
    fn launcher_matches(pid: u32, holders: &Path, run: &Path) -> bool {
        let process = PathBuf::from(format!("/proc/{pid}"));
        if !same_inode(&process.join("exe"), Path::new("/usr/bin/sudo")).unwrap_or(false)
            || !fs::metadata(&process).is_ok_and(|m| m.uid() == 0)
        {
            return false;
        }
        let expected = run.join("mvm-sender/vms");
        let argv = fs::read(process.join("cmdline")).unwrap_or_default();
        if !argv.split(|b| *b == 0).any(|arg| {
            let path = Path::new(std::ffi::OsStr::from_bytes(arg));
            path.starts_with(&expected)
                && path.file_name() == Some(std::ffi::OsStr::new("launch.sh"))
        }) {
            return false;
        }
        fs::read_dir(process.join("fd")).is_ok_and(|fds| {
            fds.flatten()
                .any(|f| same_inode(&f.path(), holders).unwrap_or(false))
        })
    }
    fn resume_owned_launcher(root: &Path) -> Result<()> {
        // SAFETY: geteuid is a scalar identity query.
        require(
            unsafe { libc::geteuid() } == 0,
            "privileged observer required",
        )?;
        let holders = root.join("installation/holders.lock");
        let run = root.join("interrupted");
        let mut observed = 0;
        for entry in fs::read_dir("/proc")?.flatten() {
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|p| p.parse::<u32>().ok())
            else {
                continue;
            };
            if launcher_matches(pid, &holders, &run) {
                let process = ExactProcess::capture(pid, || launcher_matches(pid, &holders, &run))?;
                process.signal(libc::SIGCONT)?;
                process.wait_exit()?;
                observed += 1;
            }
        }
        require(observed == 1, "expected one exact owned stopped launcher")?;
        println!("{{\"event\":\"fixture_launcher_resumed_and_exited\",\"ok\":true}}");
        Ok(())
    }
    fn command(
        host: &Path,
        installation: &Path,
        run: &Path,
        kernel: &Path,
        guest: &Path,
        size: &str,
        user: &Path,
    ) -> Result<Command> {
        let mut command = as_user(host, user);
        command
            .arg("session-installed")
            .arg(installation)
            .arg("5a".repeat(32))
            .arg(run)
            .arg(kernel)
            .arg(guest)
            .args(["one", "whole", size, "0"])
            .stdin(Stdio::null())
            .stdout(Stdio::from(File::create(run.with_extension("stdout"))?))
            .stderr(Stdio::from(File::create(run.with_extension("stderr"))?));
        Ok(command)
    }
    fn as_user(program: &Path, user: &Path) -> Command {
        let mut command = Command::new("/usr/bin/setpriv");
        command
            .args(["--reuid"])
            .arg(user)
            .args(["--regid", "kvm", "--init-groups"])
            .arg(program);
        command
    }
    fn owner_check(mode: &str, installation: &Path) -> Result<()> {
        let key = InstallationKey {
            installation: [0x5a; 32],
            profile: InstalledConfig::native_profile().profile_hash(),
        };
        if mode == "owner-initialize" {
            drop(InstallationOwner::initialize(installation, key)?);
            return Ok(());
        }
        let mut owner = InstallationOwner::open(installation, key)?;
        match mode {
            "owner-blocked" => {
                let original = fs::read(installation.join("state.bin"))?;
                let snapshot = owner.snapshot()?;
                require(
                    snapshot.phase == Phase::Active && snapshot.epoch == 1,
                    "unexpected interrupted epoch",
                )?;
                require(
                    matches!(owner.reconcile(), Err(OwnerError::Busy)),
                    "surviving backend did not exclude retirement",
                )?;
                require(
                    matches!(owner.begin(), Err(OwnerError::RecoveryRequired)),
                    "active epoch admitted successor",
                )?;
                require(
                    fs::read(installation.join("state.bin"))? == original,
                    "refusal changed original Active record",
                )?;
            }
            "owner-retire" => {
                require(
                    owner.reconcile()? == Recovery::Retired { epoch: 1 },
                    "exited backend did not allow retirement",
                )?;
                require(
                    owner.snapshot()?.phase == Phase::Clean,
                    "retirement not clean",
                )?;
            }
            "owner-final" => {
                let snapshot = owner.snapshot()?;
                require(
                    snapshot.phase == Phase::Clean && snapshot.epoch == 2,
                    "next epoch did not finish clean",
                )?;
            }
            _ => return Err("unknown measurement owner check".into()),
        }
        Ok(())
    }
    fn check_as_owner(mode: &str, installation: &Path, user: &Path) -> Result<()> {
        let mut check = ExactChild(
            as_user(&std::env::current_exe()?, user)
                .arg(mode)
                .arg(installation)
                .spawn()?,
        );
        until(
            || check.0.try_wait().is_ok_and(|s| s.is_some()),
            "owner check deadline",
        )?;
        require(check.0.wait()?.success(), "canonical owner check failed")
    }
    pub fn run() -> Result<()> {
        let args = std::env::args_os()
            .skip(1)
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        if let [mode, installation] = args.as_slice() {
            if mode == "resume-owned-launcher" {
                return resume_owned_launcher(installation);
            }
            return owner_check(mode.to_str().ok_or("invalid helper mode")?, installation);
        }
        // Privilege is confined to this explicitly admitted measurement process:
        // the production controller and all canonical owner checks run as the
        // existing login user, with the existing delegated systemd/KVM budget.
        // SAFETY: geteuid is a scalar identity query.
        require(
            unsafe { libc::geteuid() } == 0,
            "death fixture needs privileged native observer",
        )?;
        let [host, kernel, guest, root, fc, user] = args.as_slice() else {
            return Err(
                "usage: installed-native-death HOST KERNEL GUEST NEW_ROOT FIRECRACKER LOGIN_USER"
                    .into(),
            );
        };
        let identity = Command::new("/usr/bin/id").arg("-u").arg(user).output()?;
        require(
            identity.status.success() && identity.stdout.len() <= 32,
            "cannot resolve admitted login user",
        )?;
        let uid: u32 = std::str::from_utf8(&identity.stdout)?.trim().parse()?;
        require(
            uid != 0,
            "production installation needs the existing delegated nonroot login session",
        )?;
        let group = Command::new("/usr/bin/id").arg("-g").arg(user).output()?;
        require(
            group.status.success() && group.stdout.len() <= 32,
            "cannot resolve login primary group",
        )?;
        let gid: u32 = std::str::from_utf8(&group.stdout)?.trim().parse()?;
        require(
            root.is_absolute() && !root.exists(),
            "root must be new absolute path",
        )?;
        fs::create_dir(root)?;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
        std::os::unix::fs::chown(root, Some(uid), Some(gid))?;
        let root_meta = fs::metadata(root)?;
        require(
            root_meta.uid() == uid && root_meta.gid() == gid && root_meta.mode() & 0o777 == 0o700,
            "new fixture private ownership mismatch",
        )?;
        let installation = root.join("installation");
        check_as_owner("owner-initialize", &installation, user)?;
        let run = root.join("interrupted");
        let mut supervisor = ExactChild(
            command(host, &installation, &run, kernel, guest, "16777216", user)?.spawn()?,
        );
        let mut worker = None;
        until(
            || {
                if !same_inode(Path::new(&format!("/proc/{}/exe", supervisor.0.id())), host)
                    .unwrap_or(false)
                    || !fs::metadata(format!("/proc/{}", supervisor.0.id()))
                        .is_ok_and(|m| m.uid() == uid)
                {
                    return false;
                }
                for pid in child_ids(supervisor.0.id()).unwrap_or_default() {
                    if worker_matches(pid, supervisor.0.id(), host, kernel, &run) {
                        worker = ExactProcess::capture(pid, || {
                            worker_matches(pid, supervisor.0.id(), host, kernel, &run)
                        })
                        .ok();
                        if worker.is_some() {
                            return true;
                        }
                    }
                }
                false
            },
            "no exact installed sender child",
        )?;
        let mut worker = worker.ok_or("worker capture missing")?;
        let holders = installation.join("holders.lock");
        let mut backend = None;
        until(
            || {
                let Ok(entries) = fs::read_dir("/proc") else {
                    return false;
                };
                for entry in entries.flatten() {
                    let Some(pid) = entry
                        .file_name()
                        .to_str()
                        .and_then(|n| n.parse::<u32>().ok())
                    else {
                        continue;
                    };
                    if backend_matches(pid, fc, &holders, &run) {
                        backend =
                            ExactProcess::capture(pid, || backend_matches(pid, fc, &holders, &run))
                                .ok();
                        if backend.is_some() {
                            return true;
                        }
                    }
                }
                false
            },
            "no exact live KVM backend with installation holder",
        )?;
        let mut backend = backend.ok_or("backend capture missing")?;
        let stat = fs::read_to_string(format!("/proc/{}/stat", backend.pid))?;
        let parent: u32 = stat
            .rsplit_once(')')
            .and_then(|(_, s)| s.split_whitespace().nth(1))
            .ok_or("backend parent missing")?
            .parse()?;
        let mut launcher = ExactProcess::capture(parent, || {
            launcher_matches(parent, &holders, &run)
                && child_ids(parent).is_ok_and(|children| children.contains(&backend.pid))
        })?;
        // Both abrupt-exit paths now have retained exact kernel observers.
        backend.kill_on_drop = true;
        worker.kill_on_drop = true;
        launcher.kill_on_drop = true;
        let console = run
            .join("mvm-sender/vms")
            .join(format!("spiralcore-proof-{}", worker.pid))
            .join("console.log");
        until(
            || {
                fs::read_to_string(&console).is_ok_and(|s| {
                    s.lines().any(|line| {
                        line.starts_with('{') && line.contains("\"event\":\"guest_ready\"")
                    })
                })
            },
            "actual KVM guest never became ready",
        )?;
        backend.stop()?;
        worker.stop()?;
        let active = fs::read(installation.join("state.bin"))?;
        supervisor.0.kill()?;
        require(
            !supervisor.0.wait()?.success(),
            "supervisor unexpectedly succeeded",
        )?;
        worker.signal(libc::SIGKILL)?;
        worker.wait_exit()?;
        require(
            !backend.exited()?,
            "backend did not survive supervisor and worker",
        )?;
        check_as_owner("owner-blocked", &installation, user)?;
        require(
            fs::read(installation.join("state.bin"))? == active,
            "refused successor changed active state",
        )?;
        println!(
            "{{\"event\":\"surviving_backend_excludes_successor\",\"ok\":true,\"generation\":1,\"supervisor_exited\":true,\"worker_exited\":true,\"actual_kvm_holder_alive\":true}}"
        );
        backend.signal(libc::SIGKILL)?;
        backend.wait_exit()?;
        // sudo job control stops its monitor when its child is SIGSTOP'd.
        // Its inherited SH is real custody too; observe its exit before Clean.
        if !launcher.exited()? {
            launcher.signal(libc::SIGCONT)?;
        }
        launcher.wait_exit()?;
        check_as_owner("owner-retire", &installation, user)?;
        let recovered = root.join("recovered");
        let mut next = ExactChild(
            command(host, &installation, &recovered, kernel, guest, "4096", user)?.spawn()?,
        );
        until(
            || next.0.try_wait().is_ok_and(|s| s.is_some()),
            "recovered session deadline",
        )?;
        require(next.0.wait()?.success(), "next epoch transfer failed")?;
        check_as_owner("owner-final", &installation, user)?;
        println!(
            "{{\"event\":\"native_death_recovery\",\"ok\":true,\"retired_generation\":1,\"next_generation\":2,\"next_session_exit_zero\":true,\"result\":\"recovered/result.json\"}}"
        );
        Ok(())
    }
}
