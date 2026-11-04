use super::{ControlError, Result};
use std::{
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::net::UnixStream,
    },
    time::{Duration, Instant},
};

pub(super) fn remaining(deadline: Instant) -> Result<Duration> {
    let duration = deadline
        .checked_duration_since(Instant::now())
        .ok_or(ControlError::Deadline)?;
    if duration.is_zero() {
        return Err(ControlError::Deadline);
    }
    Ok(duration)
}
fn wait(stream: &UnixStream, events: i16, deadline: Instant) -> Result<()> {
    loop {
        let left = remaining(deadline)?;
        let milliseconds = left
            .as_millis()
            .saturating_add(u128::from(left.subsec_nanos() % 1_000_000 != 0))
            .min(i32::MAX as u128) as i32;
        let mut fd = libc::pollfd {
            fd: stream.as_raw_fd(),
            events,
            revents: 0,
        };
        // SAFETY: one initialized live descriptor; bounded monotonic wait.
        let result = unsafe { libc::poll(&mut fd, 1, milliseconds) };
        if result > 0 {
            remaining(deadline)?;
            return Ok(());
        }
        if result == 0 {
            return Err(ControlError::Deadline);
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error.into());
        }
    }
}
pub(super) fn read_exact(
    stream: &mut UnixStream,
    mut bytes: &mut [u8],
    deadline: Instant,
) -> Result<()> {
    while !bytes.is_empty() {
        remaining(deadline)?;
        match stream.read(bytes) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into()),
            Ok(count) => bytes = &mut bytes[count..],
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                wait(stream, libc::POLLIN, deadline)?
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.into()),
        }
    }
    remaining(deadline)?;
    Ok(())
}
pub(super) fn write_all(
    stream: &mut UnixStream,
    mut bytes: &[u8],
    deadline: Instant,
) -> Result<()> {
    while !bytes.is_empty() {
        remaining(deadline)?;
        match stream.write(bytes) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::WriteZero).into()),
            Ok(count) => bytes = &bytes[count..],
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                wait(stream, libc::POLLOUT, deadline)?
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.into()),
        }
    }
    remaining(deadline)?;
    Ok(())
}
pub(super) fn read_input(
    stream: &mut UnixStream,
    maximum: usize,
    deadline: Instant,
) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        remaining(deadline)?;
        match stream.read(&mut chunk) {
            Ok(0) if output.is_empty() => return Err(ControlError::Protocol),
            Ok(0) => {
                remaining(deadline)?;
                return Ok(output);
            }
            Ok(count) => {
                let next = output
                    .len()
                    .checked_add(count)
                    .ok_or(ControlError::Protocol)?;
                if next > maximum {
                    return Err(ControlError::Protocol);
                }
                if next > output.capacity() {
                    let capacity = output.capacity().saturating_mul(2).max(next).min(maximum);
                    output
                        .try_reserve_exact(capacity - output.len())
                        .map_err(|_| ControlError::State)?;
                }
                output.extend_from_slice(&chunk[..count]);
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                wait(stream, libc::POLLIN, deadline)?
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.into()),
        }
    }
}
pub(super) fn duplicate_stream(fd: i32) -> Result<UnixStream> {
    // SAFETY: atomic duplication rejects an invalid inherited descriptor.
    let copy = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 3) };
    if copy < 0 {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: successful fcntl returned a fresh owned descriptor.
    let owned = unsafe { OwnedFd::from_raw_fd(copy) };
    let mut address = std::mem::MaybeUninit::<libc::sockaddr_storage>::zeroed();
    let mut size = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
    // SAFETY: live descriptor, correctly sized address output and length.
    if unsafe { libc::getsockname(copy, address.as_mut_ptr().cast(), &mut size) } != 0 {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: successful getsockname initialized the family field.
    if size < std::mem::size_of::<libc::sa_family_t>() as libc::socklen_t
        || unsafe { address.assume_init() }.ss_family != libc::AF_UNIX as libc::sa_family_t
    {
        return Err(ControlError::Protocol);
    }
    let mut kind: libc::c_int = 0;
    let mut length = std::mem::size_of_val(&kind) as libc::socklen_t;
    // SAFETY: initialized scalar output and its exact length.
    if unsafe {
        libc::getsockopt(
            copy,
            libc::SOL_SOCKET,
            libc::SO_TYPE,
            (&mut kind as *mut libc::c_int).cast(),
            &mut length,
        )
    } != 0
    {
        return Err(io::Error::last_os_error().into());
    }
    if length as usize != std::mem::size_of_val(&kind) || kind != libc::SOCK_STREAM {
        return Err(ControlError::Protocol);
    }
    let stream = UnixStream::from(owned);
    stream.set_nonblocking(true)?;
    Ok(stream)
}
