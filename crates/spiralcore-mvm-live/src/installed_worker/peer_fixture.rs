//! Native socketpair/fork/exec qualification of the existing MVM peer query.
use mvm_vmm::host::uds_peer::connected_peer_pid;
use std::{
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{net::UnixStream, process::CommandExt},
    },
    process::{Child, Command, Stdio},
    time::Duration,
};

const PEER_CHILD: &str = "SPIRAL_INSTALLED_WORKER_PEER_FIXTURE";
struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn inherited_pair_reports_controller_creator_after_actual_exec() {
    let (mut parent, child_end) = UnixStream::pair().unwrap();
    parent
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let exact = format!(
        "{}::peer_child_entry",
        module_path!().split_once("::").unwrap().1
    );
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", &exact, "--nocapture", "--test-threads=1"])
        .env(PEER_CHILD, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    let fd = child_end.as_raw_fd();
    // SAFETY: only async-signal-safe descriptor syscalls run after fork. The
    // parent retains child_end until spawn returns; only child fd198 survives.
    unsafe {
        command.pre_exec(move || {
            if libc::dup2(fd, 198) < 0 || libc::fcntl(198, libc::F_SETFD, 0) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = OwnedChild(command.spawn().unwrap());
    drop(child_end);
    let mut answer = [0; 4];
    parent.read_exact(&mut answer).unwrap();
    let peer = i32::from_be_bytes(answer);
    assert_eq!(peer, i32::try_from(std::process::id()).unwrap());
    assert_ne!(peer, i32::try_from(child.0.id()).unwrap());
    assert!(child.0.wait().unwrap().success());
}

#[test]
fn peer_child_entry() {
    if std::env::var_os(PEER_CHILD).is_none() {
        return;
    }
    // SAFETY: this exact subprocess helper is invoked only after parent dup2
    // installed the private UnixStream at fd198 before exec.
    let mut stream = unsafe { UnixStream::from_raw_fd(198) };
    let peer = connected_peer_pid(&stream).unwrap();
    // SAFETY: getppid only reads this helper's live parent identity.
    assert_eq!(peer, unsafe { libc::getppid() });
    stream.write_all(&peer.to_be_bytes()).unwrap();
}
