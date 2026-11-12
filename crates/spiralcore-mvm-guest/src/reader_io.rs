//! A single absolute deadline for the normal guest-local reader operation.
//! Uses the signed Standard+mailbox profile's Unix connect and Standard's
//! read/write and readiness polling; no socket option or new socket family.
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::net::{AddressFamily, SocketAddrUnix, SocketFlags, SocketType, connect, socket_with};
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

pub(super) struct ReaderIo {
    socket: UnixStream,
    deadline: Instant,
}

fn remaining(deadline: Instant) -> io::Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "MVM reader deadline"))
}

impl ReaderIo {
    pub(super) fn connect(path: &Path, deadline: Instant) -> io::Result<Self> {
        let path = SocketAddrUnix::new(path)?;
        let socket = socket_with(
            AddressFamily::UNIX,
            SocketType::STREAM,
            SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
            None,
        )?;
        loop {
            remaining(deadline)?;
            match connect(&socket, &path) {
                Ok(()) => {
                    return Ok(Self {
                        socket: socket.into(),
                        deadline,
                    });
                }
                // Linux AF_UNIX reports EAGAIN if the local accept queue is full.
                Err(rustix::io::Errno::AGAIN) => {
                    std::thread::sleep(remaining(deadline)?.min(Duration::from_millis(5)));
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn ready(&self, event: PollFlags) -> io::Result<()> {
        loop {
            let time = Timespec::try_from(remaining(self.deadline)?).map_err(io::Error::other)?;
            let mut fds = [PollFd::new(&self.socket, event)];
            match poll(&mut fds, Some(&time)) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "MVM reader deadline",
                    ));
                }
                Ok(_) => return Ok(()),
                Err(rustix::io::Errno::INTR) => continue,
                Err(error) => return Err(error.into()),
            }
        }
    }
}
impl Read for ReaderIo {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            self.ready(PollFlags::IN)?;
            match self.socket.read(buffer) {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => continue,
                result => return result,
            }
        }
    }
}
impl Write for ReaderIo {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        loop {
            self.ready(PollFlags::OUT)?;
            match self.socket.write(buffer) {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => continue,
                result => return result,
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
