//! Canonical encrypted control sessions, with one fixed deadline and no retry.
use crate::{AccessControl, Clock, Error, LifecycleBinding, Result};
use ed25519_dalek::{SigningKey, VerifyingKey};
use mvm_contract::protocol::control::{
    ControlHello, ControlHelloFrame, ControlHelloReply, GuestCapability,
};
use mvm_core::net::session::{Session, read_sealed_frame, write_sealed_frame};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    io::{self, Read, Write},
    sync::Arc,
    time::{Duration, Instant},
};

pub trait AccessSocket: Read + Write {
    fn read_timeout(&self, timeout: Duration) -> io::Result<()>;
    fn write_timeout(&self, timeout: Duration) -> io::Result<()>;
    fn close(&self);
}
impl AccessSocket for std::net::TcpStream {
    fn read_timeout(&self, t: Duration) -> io::Result<()> {
        self.set_read_timeout(Some(t))
    }
    fn write_timeout(&self, t: Duration) -> io::Result<()> {
        self.set_write_timeout(Some(t))
    }
    fn close(&self) {
        let _ = self.shutdown(std::net::Shutdown::Both);
    }
}
#[cfg(unix)]
impl AccessSocket for std::os::unix::net::UnixStream {
    fn read_timeout(&self, t: Duration) -> io::Result<()> {
        self.set_read_timeout(Some(t))
    }
    fn write_timeout(&self, t: Duration) -> io::Result<()> {
        self.set_write_timeout(Some(t))
    }
    fn close(&self) {
        let _ = self.shutdown(std::net::Shutdown::Both);
    }
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct WireCounters {
    pub sent: u64,
    pub received: u64,
    pub frames_sent: u64,
    pub frames_received: u64,
}
#[derive(Clone)]
pub(crate) struct IoGuard {
    pub control: AccessControl,
    pub binding: LifecycleBinding,
    pub clock: Arc<dyn Clock>,
    pub deadline: Instant,
    pub unix_deadline: chrono::DateTime<chrono::Utc>,
    pub extra_current: Option<Arc<dyn Fn() -> Result<()> + Send + Sync>>,
}
impl IoGuard {
    pub(crate) fn check(&self) -> Result<()> {
        if let Some(current) = &self.extra_current {
            current()?;
        }
        if self.control.snapshot()? != self.binding {
            return Err(Error::Epoch);
        }
        let now = self.clock.now();
        if now.unix >= self.unix_deadline
            || now.monotonic >= self.deadline
            || Instant::now() >= self.deadline
        {
            return Err(Error::Expired);
        }
        Ok(())
    }
}
struct BoundedIo<'a, S> {
    socket: &'a mut S,
    counts: &'a mut WireCounters,
    deadline: Instant,
    guard: Option<&'a IoGuard>,
}
impl<S: AccessSocket> BoundedIo<'_, S> {
    fn remaining(&self) -> io::Result<Duration> {
        if let Some(guard) = self.guard {
            guard
                .check()
                .map_err(|_| io::Error::from(io::ErrorKind::PermissionDenied))?;
        }
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|t| !t.is_zero())
            .map(|t| t.min(Duration::from_millis(50)))
            .ok_or_else(|| io::ErrorKind::TimedOut.into())
    }
}
impl<S: AccessSocket> Read for BoundedIo<'_, S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let timeout = self.remaining()?;
            self.socket.read_timeout(timeout)?;
            match self.socket.read(buf) {
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::Interrupted
                            | io::ErrorKind::WouldBlock
                            | io::ErrorKind::TimedOut
                    ) =>
                {
                    continue;
                }
                Ok(n) => {
                    self.counts.received = self
                        .counts
                        .received
                        .checked_add(n as u64)
                        .ok_or(io::ErrorKind::InvalidData)?;
                    return Ok(n);
                }
                Err(e) => return Err(e),
            }
        }
    }
}
impl<S: AccessSocket> Write for BoundedIo<'_, S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        loop {
            let timeout = self.remaining()?;
            self.socket.write_timeout(timeout)?;
            match self.socket.write(buf) {
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::Interrupted
                            | io::ErrorKind::WouldBlock
                            | io::ErrorKind::TimedOut
                    ) =>
                {
                    continue;
                }
                Ok(n) => {
                    self.counts.sent = self
                        .counts
                        .sent
                        .checked_add(n as u64)
                        .ok_or(io::ErrorKind::InvalidData)?;
                    return Ok(n);
                }
                Err(e) => return Err(e),
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        self.remaining()?;
        self.socket.flush()
    }
}
pub struct OwnerClient<S: AccessSocket> {
    socket: S,
    session: Session,
    counts: WireCounters,
    closed: bool,
    object_negotiated: bool,
}
impl<S: AccessSocket> OwnerClient<S> {
    pub fn connect(
        socket: S,
        instance: &str,
        key: SigningKey,
        peer: &VerifyingKey,
        deadline: Instant,
    ) -> Result<Self> {
        Self::connect_inner(socket, instance, key, peer, deadline, None, false)
    }
    #[cfg(feature = "object-v1")]
    pub fn connect_object(
        socket: S,
        instance: &str,
        key: SigningKey,
        peer: &VerifyingKey,
        deadline: Instant,
    ) -> Result<Self> {
        Self::connect_inner(socket, instance, key, peer, deadline, None, true)
    }
    pub(crate) fn connect_guarded(
        socket: S,
        instance: &str,
        key: SigningKey,
        peer: &VerifyingKey,
        guard: &IoGuard,
    ) -> Result<Self> {
        Self::connect_inner(
            socket,
            instance,
            key,
            peer,
            guard.deadline,
            Some(guard),
            false,
        )
    }
    #[cfg(feature = "object-v1")]
    pub(crate) fn connect_object_guarded(
        socket: S,
        instance: &str,
        key: SigningKey,
        peer: &VerifyingKey,
        guard: &IoGuard,
    ) -> Result<Self> {
        Self::connect_inner(
            socket,
            instance,
            key,
            peer,
            guard.deadline,
            Some(guard),
            true,
        )
    }
    fn connect_inner(
        mut socket: S,
        instance: &str,
        key: SigningKey,
        peer: &VerifyingKey,
        deadline: Instant,
        guard: Option<&IoGuard>,
        object: bool,
    ) -> Result<Self> {
        let mut counts = WireCounters::default();
        let result = Session::host(
            &mut BoundedIo {
                socket: &mut socket,
                counts: &mut counts,
                deadline,
                guard,
            },
            instance,
            key,
        );
        let (session, actual) = match result {
            Ok(result) => result,
            Err(_) => {
                socket.close();
                return Err(Error::Transport);
            }
        };
        if &actual != peer {
            socket.close();
            return Err(Error::Signature);
        }
        let mut client = Self {
            socket,
            session,
            counts,
            closed: false,
            object_negotiated: false,
        };
        let result = (|| {
            // Version-only first: a frozen v2 peer can return ProtocolMismatch
            // without needing to deserialize an unknown capability enum value.
            let mut required = vec![GuestCapability::OwnerAccessV1];
            if object {
                required.push(GuestCapability::OwnerAccessObjectV1);
            }
            for requested_capabilities in [Vec::new(), required] {
                client.write_message(
                    &ControlHelloFrame {
                        hello: ControlHello {
                            host_protocol_version: 3,
                            min_supported_version: 3,
                            host_version: env!("CARGO_PKG_VERSION").into(),
                            requested_capabilities: requested_capabilities.clone(),
                        },
                    },
                    deadline,
                    guard,
                )?;
                let reply: ControlHelloReply = client.read_message(deadline, guard, 16384)?;
                match reply {
                    ControlHelloReply::ProtocolHelloAck {
                        agent_protocol_version,
                        min_supported_version,
                        capabilities,
                        ..
                    } if agent_protocol_version >= 3
                        && min_supported_version <= 3
                        && requested_capabilities
                            .iter()
                            .all(|cap| capabilities.contains(cap)) => {}
                    _ => return Err(Error::Unsupported),
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            client.close();
            return Err(error);
        }
        client.object_negotiated = object;
        Ok(client)
    }
    pub(crate) fn write_message<T: Serialize>(
        &mut self,
        value: &T,
        deadline: Instant,
        guard: Option<&IoGuard>,
    ) -> Result<()> {
        if self.closed {
            return Err(Error::Closed);
        }
        if let Some(guard) = guard {
            guard.check()?;
        }
        let plaintext = serde_json::to_vec(value).map_err(|_| Error::Policy)?;
        if plaintext.len()
            > if self.object_negotiated {
                256 * 1024
            } else {
                70 * 1024
            }
        {
            return Err(Error::Limit);
        }
        let result = (|| {
            let sealed = self
                .session
                .seal(&plaintext)
                .map_err(|_| Error::Transport)?;
            write_sealed_frame(
                &mut BoundedIo {
                    socket: &mut self.socket,
                    counts: &mut self.counts,
                    deadline,
                    guard,
                },
                &sealed,
            )
            .map_err(|_| {
                guard
                    .and_then(|g| g.check().err())
                    .unwrap_or(Error::Transport)
            })?;
            Ok(())
        })();
        if result.is_err() {
            self.close();
        } else {
            self.counts.frames_sent += 1;
        }
        result
    }
    pub(crate) fn read_message<T: DeserializeOwned>(
        &mut self,
        deadline: Instant,
        guard: Option<&IoGuard>,
        limit: usize,
    ) -> Result<T> {
        if self.closed {
            return Err(Error::Closed);
        }
        let result = (|| {
            let sealed = read_sealed_frame(
                &mut BoundedIo {
                    socket: &mut self.socket,
                    counts: &mut self.counts,
                    deadline,
                    guard,
                },
                limit.min(if self.object_negotiated {
                    256 * 1024 + 8192
                } else {
                    80 * 1024
                }),
            )
            .map_err(|_| {
                guard
                    .and_then(|g| g.check().err())
                    .unwrap_or(Error::Transport)
            })?;
            let bytes = self.session.open(&sealed).map_err(|_| Error::Signature)?;
            serde_json::from_slice(&bytes).map_err(|_| Error::Policy)
        })();
        if result.is_err() {
            self.close();
        } else {
            self.counts.frames_received += 1;
        }
        result
    }
    pub fn wire_counters(&self) -> WireCounters {
        self.counts
    }
    #[cfg(feature = "object-v1")]
    pub(crate) fn object_negotiated(&self) -> bool {
        self.object_negotiated
    }
    pub fn close(&mut self) {
        self.closed = true;
        self.socket.close();
    }
}
impl<S: AccessSocket> Drop for OwnerClient<S> {
    fn drop(&mut self) {
        self.socket.close();
    }
}
