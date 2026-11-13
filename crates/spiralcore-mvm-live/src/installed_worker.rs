//! Private host-local installed worker grants; never an MVM network envelope.
#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), test))]
use crate::installed_owner::{HolderHandoff, HolderLease};
use crate::{
    installed_owner::{InstallationKey, OwnerError},
    installed_supervisor::WorkerBinding,
};
#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), test))]
use mvm_vmm::host::{
    process_exit::{ProcessExitObserver, ProcessExitWait},
    uds_peer::connected_peer_pid,
};
#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), test))]
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    fmt, io,
    net::Shutdown,
    os::unix::{ffi::OsStrExt, net::UnixStream},
    path::Component,
    time::{Duration, Instant},
};

#[path = "installed_worker/codec.rs"]
mod codec;
#[path = "installed_worker/io_util.rs"]
mod io_util;
use codec::{Record, read_record, write_record};
const START_LIMIT: Duration = Duration::from_secs(40);
const MAX_INITRAMFS: usize = 64 * 1024 * 1024;
const MAX_ROOT: usize = 896;
#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), test))]
const CONTROL_FD: i32 = 198;
#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), test))]
static CHILD_TAKEN: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub(crate) enum ControlError {
    Io(io::Error),
    Owner(OwnerError),
    Protocol,
    State,
    Deadline,
    #[cfg(any(all(target_os = "macos", target_arch = "aarch64"), test))]
    Peer,
}
impl fmt::Display for ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "installed worker I/O: {error}"),
            Self::Owner(error) => write!(f, "installed worker owner: {error}"),
            Self::Protocol => f.write_str("installed worker protocol refusal"),
            Self::State => f.write_str("installed worker phase refusal"),
            Self::Deadline => f.write_str("installed worker deadline"),
            #[cfg(any(all(target_os = "macos", target_arch = "aarch64"), test))]
            Self::Peer => f.write_str("installed worker peer unavailable"),
        }
    }
}
impl std::error::Error for ControlError {}
impl From<io::Error> for ControlError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<OwnerError> for ControlError {
    fn from(e: OwnerError) -> Self {
        Self::Owner(e)
    }
}
type Result<T, E = ControlError> = std::result::Result<T, E>;

impl WorkerBinding {
    fn validate(&self, role: u8) -> Result<()> {
        if !(1..=3).contains(&role)
            || self.role != role
            || self.slot >= 2
            || self.serial == 0
            || self.generation == 0
            || self.installation == [0; 32]
            || self.profile
                != crate::installed_supervisor::InstalledConfig::native_profile().profile_hash()
            || !self.root.is_absolute()
            || self.root.as_os_str().as_bytes().len() > MAX_ROOT
            || self.root.as_os_str().as_bytes().contains(&0)
            || self
                .root
                .components()
                .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
        {
            return Err(ControlError::Protocol);
        }
        Ok(())
    }
}

fn same_binding(left: &WorkerBinding, right: &WorkerBinding) -> bool {
    left == right && left.root.as_os_str().as_bytes() == right.root.as_os_str().as_bytes()
}

#[derive(PartialEq, Eq)]
enum ParentPhase {
    Hello,
    Binding,
    Ready,
    Input,
    BootChallenge,
    Go,
    Done,
    Failed,
}
pub(crate) struct ParentControl {
    stream: UnixStream,
    deadline: Instant,
    phase: ParentPhase,
    first: Option<[u8; 32]>,
    second: Option<[u8; 32]>,
    original: Option<WorkerBinding>,
}
impl ParentControl {
    pub(crate) fn pair() -> Result<(Self, UnixStream)> {
        let deadline = Instant::now()
            .checked_add(START_LIMIT)
            .ok_or(ControlError::Deadline)?;
        let (stream, child) = UnixStream::pair()?;
        stream.set_nonblocking(true)?;
        Ok((
            Self {
                stream,
                deadline,
                phase: ParentPhase::Hello,
                first: None,
                second: None,
                original: None,
            },
            child,
        ))
    }
    fn run<T>(&mut self, action: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let result = action(self);
        if result.is_err() {
            self.phase = ParentPhase::Failed;
            let _ = self.stream.shutdown(Shutdown::Both);
        }
        result
    }
    fn require(&self, phase: ParentPhase) -> Result<()> {
        io_util::remaining(self.deadline)?;
        if self.phase != phase {
            return Err(ControlError::State);
        }
        Ok(())
    }
    pub(crate) fn receive_challenge(&mut self) -> Result<()> {
        self.run(|this| {
            this.require(ParentPhase::Hello)?;
            let Record::Hello(nonce) = read_record(&mut this.stream, this.deadline)? else {
                return Err(ControlError::Protocol);
            };
            this.first = Some(nonce);
            this.phase = ParentPhase::Binding;
            Ok(())
        })
    }
    pub(crate) fn send_binding(&mut self, binding: &WorkerBinding) -> Result<()> {
        self.run(|this| {
            this.require(ParentPhase::Binding)?;
            binding.validate(binding.role)?;
            write_record(
                &mut this.stream,
                this.deadline,
                &Record::Binding(this.first.ok_or(ControlError::State)?, binding.clone()),
            )?;
            this.original = Some(binding.clone());
            this.phase = ParentPhase::Ready;
            Ok(())
        })
    }
    pub(crate) fn wait_ready(&mut self) -> Result<()> {
        self.run(|this| {
            this.require(ParentPhase::Ready)?;
            let Record::Ready(nonce) = read_record(&mut this.stream, this.deadline)? else {
                return Err(ControlError::Protocol);
            };
            if Some(nonce) != this.first {
                return Err(ControlError::Protocol);
            }
            this.phase = ParentPhase::Input;
            Ok(())
        })
    }
    pub(crate) fn write_initrd(&mut self, writer: &mut UnixStream, input: &[u8]) -> Result<()> {
        let result = self.run(|this| {
            this.require(ParentPhase::Input)?;
            if input.is_empty() || input.len() > MAX_INITRAMFS {
                return Err(ControlError::Protocol);
            }
            writer.set_nonblocking(true)?;
            io_util::write_all(writer, input, this.deadline)?;
            writer.shutdown(Shutdown::Write)?;
            this.phase = ParentPhase::BootChallenge;
            Ok(())
        });
        if result.is_err() {
            let _ = writer.shutdown(Shutdown::Both);
        }
        result
    }
    pub(crate) fn receive_boot_challenge(&mut self) -> Result<()> {
        self.run(|this| {
            this.require(ParentPhase::BootChallenge)?;
            let Record::BootChallenge(nonce) = read_record(&mut this.stream, this.deadline)? else {
                return Err(ControlError::Protocol);
            };
            if Some(nonce) == this.first {
                return Err(ControlError::Protocol);
            }
            this.second = Some(nonce);
            this.phase = ParentPhase::Go;
            Ok(())
        })
    }
    pub(crate) fn send_go(&mut self, binding: &WorkerBinding) -> Result<()> {
        self.run(|this| {
            this.require(ParentPhase::Go)?;
            binding.validate(binding.role)?;
            if !this
                .original
                .as_ref()
                .is_some_and(|original| same_binding(original, binding))
            {
                return Err(ControlError::Protocol);
            }
            write_record(
                &mut this.stream,
                this.deadline,
                &Record::Go(this.second.ok_or(ControlError::State)?, binding.clone()),
            )?;
            this.phase = ParentPhase::Done;
            Ok(())
        })
    }
}

#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), test))]
#[derive(PartialEq, Eq)]
enum ChildPhase {
    Input,
    Boot,
    Authorized,
    Failed,
}
#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), test))]
pub(crate) struct InstalledWorkerContext {
    binding: WorkerBinding,
    holder: HolderLease,
    stream: UnixStream,
    observer: ProcessExitObserver,
    deadline: Instant,
    first: [u8; 32],
    phase: ChildPhase,
}
#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), test))]
impl InstalledWorkerContext {
    pub(crate) fn from_inherited_fd(expected_role: u8) -> Result<Self> {
        let deadline = Instant::now()
            .checked_add(START_LIMIT)
            .ok_or(ControlError::Deadline)?;
        if CHILD_TAKEN.swap(true, Ordering::AcqRel) {
            return Err(ControlError::State);
        }
        let mut stream = io_util::duplicate_stream(CONTROL_FD)?;
        // This entry is called once before other worker threads. Releasing the
        // inherited descriptor prevents an unprotected duplicate reaching FC.
        // SAFETY: duplicate_stream proved the fixed inherited fd is valid.
        if unsafe { libc::close(CONTROL_FD) } != 0 {
            return Err(io::Error::last_os_error().into());
        }
        let result = (|| {
            if !(1..=3).contains(&expected_role) {
                return Err(ControlError::Protocol);
            }
            let peer = connected_peer_pid(&stream).map_err(|_| ControlError::Peer)?;
            // SAFETY: getppid only reads the live worker's kernel parent.
            if peer != unsafe { libc::getppid() } {
                return Err(ControlError::Peer);
            }
            let observer = ProcessExitObserver::arm(peer).map_err(|_| ControlError::Peer)?;
            check_peer(&observer, deadline)?;
            let first = rand::random::<[u8; 32]>();
            write_record(&mut stream, deadline, &Record::Hello(first))?;
            let Record::Binding(nonce, binding) = read_record(&mut stream, deadline)? else {
                return Err(ControlError::Protocol);
            };
            if nonce != first {
                return Err(ControlError::Protocol);
            }
            binding.validate(expected_role)?;
            check_peer(&observer, deadline)?;
            let holder = HolderLease::acquire(
                &binding.root,
                InstallationKey {
                    installation: binding.installation,
                    profile: binding.profile,
                },
                binding.generation,
            )?;
            check_peer(&observer, deadline)?;
            write_record(&mut stream, deadline, &Record::Ready(first))?;
            Ok((binding, holder, observer, first))
        })();
        match result {
            Ok((binding, holder, observer, first)) => Ok(Self {
                binding,
                holder,
                observer,
                first,
                stream,
                deadline,
                phase: ChildPhase::Input,
            }),
            Err(error) => {
                let _ = stream.shutdown(Shutdown::Both);
                Err(error)
            }
        }
    }
    fn run<T>(&mut self, action: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let result = action(self);
        if result.is_err() {
            self.phase = ChildPhase::Failed;
            let _ = self.stream.shutdown(Shutdown::Both);
        }
        result
    }
    pub(crate) fn read_initrd(&mut self) -> Result<Vec<u8>> {
        self.run(|this| {
            if this.phase != ChildPhase::Input {
                return Err(ControlError::State);
            }
            check_peer(&this.observer, this.deadline)?;
            let mut input = io_util::duplicate_stream(0)?;
            let bytes = match io_util::read_input(&mut input, MAX_INITRAMFS, this.deadline) {
                Ok(bytes) => bytes,
                Err(error) => {
                    let _ = input.shutdown(Shutdown::Both);
                    return Err(error);
                }
            };
            check_peer(&this.observer, this.deadline)?;
            this.phase = ChildPhase::Boot;
            Ok(bytes)
        })
    }
    pub(crate) fn authorize_boot(&mut self) -> Result<()> {
        self.run(|this| {
            if this.phase != ChildPhase::Boot {
                return Err(ControlError::State);
            }
            check_peer(&this.observer, this.deadline)?;
            let second = rand::random::<[u8; 32]>();
            if second == this.first {
                return Err(ControlError::Protocol);
            }
            write_record(
                &mut this.stream,
                this.deadline,
                &Record::BootChallenge(second),
            )?;
            let Record::Go(nonce, binding) = read_record(&mut this.stream, this.deadline)? else {
                return Err(ControlError::Protocol);
            };
            binding.validate(this.binding.role)?;
            if nonce != second || !same_binding(&binding, &this.binding) {
                return Err(ControlError::Protocol);
            }
            check_peer(&this.observer, this.deadline)?;
            this.phase = ChildPhase::Authorized;
            Ok(())
        })
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    pub(crate) fn binding(&self) -> &WorkerBinding {
        &self.binding
    }
    pub(crate) fn check_before_backend(&self) -> Result<()> {
        if self.phase != ChildPhase::Authorized {
            return Err(ControlError::State);
        }
        check_peer(&self.observer, self.deadline)
    }
    pub(crate) fn holder_handoff(&self) -> Result<HolderHandoff> {
        self.check_before_backend()?;
        Ok(self.holder.prepare_handoff()?)
    }
}
#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), test))]
fn check_peer(observer: &ProcessExitObserver, deadline: Instant) -> Result<()> {
    io_util::remaining(deadline)?;
    match observer
        .wait_event(Instant::now())
        .map_err(|_| ControlError::Peer)?
    {
        ProcessExitWait::Exited => Err(ControlError::Peer),
        ProcessExitWait::TimedOut => Ok(()),
    }
}

#[cfg(test)]
#[path = "installed_worker/peer_fixture.rs"]
mod peer_fixture;
#[cfg(test)]
#[path = "installed_worker/tests.rs"]
mod tests;
