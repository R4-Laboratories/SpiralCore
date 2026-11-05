//! One Linux mailbox socket, its original worker, and the caller's charge.
use std::{
    fs::{self, File},
    io,
    net::Shutdown,
    os::{
        fd::{AsRawFd, RawFd},
        unix::{
            fs::{FileTypeExt, MetadataExt},
            net::{UnixListener, UnixStream},
        },
    },
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const SOCKET_NAME: &str = "mailbox.sock";
const STACK_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy)]
enum SocketState {
    Empty,
    Uncertain,
    Bound { device: u64, inode: u64 },
    Unlinked,
}

struct BoundSocket {
    directory: File,
    path: PathBuf,
    state: SocketState,
}

impl BoundSocket {
    fn verify_directory(&self) -> io::Result<()> {
        let held = self.directory.metadata()?;
        let named = fs::symlink_metadata(self.path.parent().expect("absolute socket path"))?;
        // SAFETY: geteuid has no pointer arguments or caller preconditions.
        let uid = unsafe { libc::geteuid() };
        if !named.is_dir()
            || (held.dev(), held.ino()) != (named.dev(), named.ino())
            || held.uid() != uid
            || held.mode() & 0o7777 != 0o700
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "original private mailbox directory changed",
            ));
        }
        Ok(())
    }

    fn descriptor_path(&self) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.directory.as_raw_fd())).join(SOCKET_NAME)
    }

    fn cleanup(&mut self) -> io::Result<()> {
        if matches!(self.state, SocketState::Empty) {
            return Ok(());
        }
        self.verify_directory()?;
        let pinned = self.descriptor_path();
        match self.state {
            SocketState::Uncertain => {
                return Err(io::Error::other(
                    "original mailbox socket identity unavailable",
                ));
            }
            SocketState::Bound { device, inode } => {
                check_socket(&pinned, device, inode)?;
                fs::remove_file(&pinned)?;
                self.state = SocketState::Unlinked;
            }
            SocketState::Unlinked => {}
            SocketState::Empty => unreachable!(),
        }
        self.directory.sync_all()?;
        self.verify_directory()?;
        match fs::symlink_metadata(pinned) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
            Ok(_) => Err(io::Error::other("mailbox socket name is no longer empty")),
        }
    }
}

fn check_socket(path: &Path, device: u64, inode: u64) -> io::Result<()> {
    let socket = fs::symlink_metadata(path)?;
    if !socket.file_type().is_socket() || (socket.dev(), socket.ino()) != (device, inode) {
        return Err(io::Error::other("original mailbox socket identity changed"));
    }
    Ok(())
}

#[derive(Default)]
struct Control {
    stopped: bool,
    accepted: Option<UnixStream>,
}

struct Custody<R> {
    socket: BoundSocket,
    listener: Option<UnixListener>,
    wake: Option<UnixStream>,
    waiter: Option<UnixStream>,
    control: Arc<Mutex<Control>>,
    thread: Option<JoinHandle<io::Result<()>>>,
    result: Option<io::Result<()>>,
    panicked: bool,
    retired: bool,
    _charge: R,
}

/// The caller retains this owner before start. Directory-entry mutation must
/// be exclusive to the cooperating original Runtime owner.
#[must_use]
pub(super) struct MailboxSocketWorker<R> {
    custody: Option<Box<Custody<R>>>,
}

impl<R> MailboxSocketWorker<R> {
    pub(super) fn prepare(directory: File, directory_path: &Path, charge: R) -> io::Result<Self> {
        if !directory_path.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "mailbox directory must be absolute",
            ));
        }
        let socket = BoundSocket {
            directory,
            path: directory_path.join(SOCKET_NAME),
            state: SocketState::Empty,
        };
        socket.verify_directory()?;
        let (wake, waiter) = UnixStream::pair()?;
        Ok(Self {
            custody: Some(Box::new(Custody {
                socket,
                listener: None,
                wake: Some(wake),
                waiter: Some(waiter),
                control: Arc::new(Mutex::new(Control::default())),
                thread: None,
                result: None,
                panicked: false,
                retired: false,
                _charge: charge,
            })),
        })
    }

    pub(super) fn start(
        &mut self,
        job: impl FnOnce(UnixStream) -> io::Result<()> + Send + 'static,
    ) -> io::Result<()> {
        self.start_checked(job, |path| UnixListener::bind(path), || Ok(()))
    }

    pub(super) fn start_checked(
        &mut self,
        job: impl FnOnce(UnixStream) -> io::Result<()> + Send + 'static,
        bind: impl FnOnce(&Path) -> io::Result<UnixListener>,
        after_bind: impl FnOnce() -> io::Result<()>,
    ) -> io::Result<()> {
        let state = self.custody.as_mut().expect("owned mailbox custody");
        if state.retired || !matches!(state.socket.state, SocketState::Empty) {
            return Err(io::Error::other("mailbox socket startup already attempted"));
        }
        state.socket.verify_directory()?;
        let path = state.socket.descriptor_path();
        match fs::symlink_metadata(&path) {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    "mailbox socket name is occupied",
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        // UnixListener::bind includes listen(2), which can fail after
        // bind(2) already created the pathname.
        state.socket.state = SocketState::Uncertain;
        match bind(&path) {
            Ok(listener) => state.listener = Some(listener),
            Err(error) => {
                if state.socket.verify_directory().is_ok()
                    && matches!(fs::symlink_metadata(&path), Err(error) if error.kind() == io::ErrorKind::NotFound)
                {
                    state.socket.state = SocketState::Empty;
                }
                return Err(error);
            }
        }
        after_bind()?;
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.file_type().is_socket() {
            return Err(io::Error::other(
                "original mailbox socket identity unavailable",
            ));
        }
        let (device, inode) = (metadata.dev(), metadata.ino());
        state.socket.state = SocketState::Bound { device, inode };
        state.socket.verify_directory()?;
        check_socket(&state.socket.path, device, inode)?;

        let listener = state
            .listener
            .as_ref()
            .expect("original mailbox listener")
            .try_clone()?;
        listener.set_nonblocking(true)?;
        let waiter = state.waiter.take().expect("original mailbox accept wake");
        let control = Arc::clone(&state.control);
        state.thread = Some(
            thread::Builder::new()
                .name("installed-mailbox".into())
                .stack_size(STACK_BYTES)
                .spawn(move || run(listener, waiter, control, job))?,
        );
        state.listener.take();
        Ok(())
    }

    pub(super) fn socket_path(&self) -> io::Result<&Path> {
        let state = self.custody.as_ref().expect("owned mailbox custody");
        let ready = state
            .thread
            .as_ref()
            .is_some_and(|worker| !worker.is_finished());
        let stopping = state
            .control
            .lock()
            .map_err(|_| io::Error::other("mailbox control poisoned"))?
            .stopped;
        let SocketState::Bound { device, inode } = state.socket.state else {
            return Err(io::Error::other("mailbox worker has no bound socket"));
        };
        if !ready || stopping {
            return Err(io::Error::other("original mailbox worker is unavailable"));
        }
        state.socket.verify_directory()?;
        check_socket(&state.socket.path, device, inode)?;
        Ok(&state.socket.path)
    }

    /// The deadline bounds waiting to join. Filesystem synchronization is a
    /// separate blocking operation. A refusal keeps the exact owner for retry.
    pub(super) fn stop(&mut self, join_deadline: Instant) -> io::Result<()> {
        let state = self.custody.as_mut().expect("owned mailbox custody");
        if state.retired {
            return Ok(());
        }
        state.signal_stop()?;
        while state
            .thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
        {
            let remaining = join_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "original mailbox worker still running",
                ));
            }
            thread::park_timeout(remaining.min(Duration::from_millis(1)));
        }
        if let Some(thread) = state.thread.take() {
            match thread.join() {
                Ok(result) => state.result = Some(result),
                Err(_) => state.panicked = true,
            }
        }
        if state.panicked {
            return Err(io::Error::other("original mailbox worker panicked"));
        }
        state.socket.cleanup()?;
        state.listener.take();
        state.wake.take();
        state.waiter.take();
        state.retired = true;
        Ok(())
    }

    pub(super) fn take_worker_result(&mut self) -> Option<io::Result<()>> {
        self.custody
            .as_mut()
            .expect("owned mailbox custody")
            .result
            .take()
    }
}

impl<R> Drop for MailboxSocketWorker<R> {
    fn drop(&mut self) {
        let Some(custody) = self.custody.take() else {
            return;
        };
        if !custody.retired && !matches!(custody.socket.state, SocketState::Empty) {
            let _ = custody.signal_stop();
            // Preserve the original thread handle, namespace and charge. The
            // active Runtime slot will retain the owner across stop refusals.
            std::mem::forget(custody);
        }
    }
}

impl<R> Custody<R> {
    fn signal_stop(&self) -> io::Result<()> {
        let mut state = self
            .control
            .lock()
            .map_err(|_| io::Error::other("mailbox control poisoned"))?;
        state.stopped = true;
        let wake = self
            .wake
            .as_ref()
            .map_or(Ok(()), |s| shutdown(s, Shutdown::Write));
        let accepted = state
            .accepted
            .as_ref()
            .map_or(Ok(()), |s| shutdown(s, Shutdown::Both));
        wake.and(accepted)
    }
}

fn shutdown(socket: &UnixStream, how: Shutdown) -> io::Result<()> {
    match socket.shutdown(how) {
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotConnected | io::ErrorKind::BrokenPipe
            ) =>
        {
            Ok(())
        }
        result => result,
    }
}

struct Accepted(Arc<Mutex<Control>>);

impl Drop for Accepted {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.lock() {
            state.accepted.take();
        }
    }
}

fn run(
    listener: UnixListener,
    wake: UnixStream,
    control: Arc<Mutex<Control>>,
    job: impl FnOnce(UnixStream) -> io::Result<()>,
) -> io::Result<()> {
    loop {
        if !wait_for_peer(listener.as_raw_fd(), wake.as_raw_fd())? {
            return Ok(());
        }
        let (socket, _) = match listener.accept() {
            Ok(accepted) => accepted,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => continue,
            Err(error) => return Err(error),
        };
        socket.set_nonblocking(false)?;
        {
            let mut state = control
                .lock()
                .map_err(|_| io::Error::other("mailbox control poisoned"))?;
            if state.stopped {
                return Ok(());
            }
            state.accepted = Some(socket.try_clone()?);
        }
        let _accepted = Accepted(Arc::clone(&control));
        return job(socket);
    }
}

fn wait_for_peer(listener: RawFd, wake: RawFd) -> io::Result<bool> {
    loop {
        let mut inputs = [
            libc::pollfd {
                fd: listener,
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: wake,
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        // SAFETY: the worker owns both live descriptors and the array holds
        // initialized writable pollfd entries.
        let status = unsafe { libc::poll(inputs.as_mut_ptr(), inputs.len() as libc::nfds_t, -1) };
        if status < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        if inputs[1].revents != 0 {
            return Ok(false);
        }
        if inputs[0].revents & libc::POLLIN != 0 {
            return Ok(true);
        }
        return Err(io::Error::other("original mailbox listener poll failed"));
    }
}
