//! Canonical object-v1 transport; codecs, authority and publication stay in the core.
use crate::{Binding, Cancellation, Counters, Error, MAX_BLOCK, Result, Rx, TcpMapping, Tx};
use spiralcore_transfer::{
    Id,
    client::Stats,
    endpoint::{Authority, Context},
    protocol::Frame,
    v1::{Capabilities, NegotiatedProfile, ObjectEndpoint, ObjectReceipt, ObjectSender},
};
use std::{
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};

#[derive(Debug, PartialEq, Eq)]
pub enum ServeEvent {
    Handled {
        request: Frame,
        reply: Frame,
        purpose: crate::Purpose,
    },
    HalfClose,
}
pub struct ObjectChannel {
    pub(crate) authenticated_role: Option<crate::Role>,
    pub(crate) tx: Tx,
    pub(crate) rx: Rx,
    pub(crate) binding: Binding,
    pub(crate) timeout: Duration,
    profile: NegotiatedProfile,
    cancellation: Cancellation,
}
impl ObjectChannel {
    /// The explicit enrollment must match this actual mutual TLS session.
    pub fn from_authenticated(
        socket: crate::tls::TlsCarrier,
        scope: &crate::scope::ScopeAuthority,
        timeout: Duration,
        role: crate::Role,
        local: Capabilities,
        trusted_peer: Capabilities,
    ) -> Result<Self> {
        scope.check_carrier(&socket, role)?;
        let profile = local.negotiate(&trusted_peer)?;
        if profile.max_object_bytes() > 65536 {
            return Err(Error::Limit);
        }
        let mapping = TcpMapping::with_authenticated(socket, scope, timeout, role)?;
        let cancellation = mapping.cancellation();
        let (tx, rx) = mapping.split();
        Ok(Self {
            tx,
            rx,
            binding: scope.binding()?,
            timeout,
            profile,
            cancellation,
            authenticated_role: Some(role),
        })
    }
    /// Capabilities must be authenticated or explicitly trusted-pinned by the caller.
    pub fn from_connected(
        socket: TcpStream,
        binding: Binding,
        peer: SocketAddr,
        timeout: Duration,
        local: Capabilities,
        trusted_peer: Capabilities,
    ) -> Result<Self> {
        let profile = local.negotiate(&trusted_peer)?;
        if profile.max_object_bytes() > 65536 {
            return Err(Error::Limit);
        }
        let mapping = TcpMapping::with_profile(socket, binding, peer, timeout, true)?;
        let cancellation = mapping.cancellation();
        let (tx, rx) = mapping.split();
        Ok(Self {
            authenticated_role: None,
            tx,
            rx,
            binding,
            timeout,
            profile,
            cancellation,
        })
    }
    pub fn profile(&self) -> NegotiatedProfile {
        self.profile
    }
    pub fn counters(&self) -> (Counters, Counters) {
        (self.tx.counters(), self.rx.counters())
    }
    pub fn cancellation(&self) -> Cancellation {
        self.cancellation.clone()
    }
    pub(crate) fn check_context(&self, context: &Context) -> Result<()> {
        if context.namespace()? != self.binding.flow
            || context.policy_epoch != 0
            || context.execution_epoch != 0
        {
            return Err(Error::InvalidBinding);
        }
        Ok(())
    }
    fn deadline(&self) -> Result<Instant> {
        Instant::now().checked_add(self.timeout).ok_or(Error::Limit)
    }
    fn exchange(
        &mut self,
        frames: &[Frame],
        until: Instant,
        purpose: crate::Purpose,
    ) -> Result<Vec<Frame>> {
        if frames.is_empty() || frames.len() > 8 {
            return Err(Error::Limit);
        }
        for frame in frames {
            if !matches!(frame.kind, 1..=6) {
                return Err(Error::Protocol);
            }
            frame.encode()?;
        }
        let mut replies = Vec::with_capacity(frames.len());
        for frame in frames {
            self.tx.send_object(frame, purpose, until)?;
            let crate::ObjectRecord::Frame(reply_purpose, reply) = self.rx.receive_object(until)?
            else {
                return Err(Error::Closed);
            };
            if reply_purpose != purpose {
                return Err(Error::Protocol);
            }
            if reply.transfer != frame.transfer
                || reply.seq != frame.seq
                || !matches!(reply.kind, 101..=105 | 127)
            {
                return Err(Error::Protocol);
            }
            let terminal = matches!(reply.kind, 103 | 127);
            replies.push(reply);
            if terminal {
                break;
            }
        }
        Ok(replies)
    }
    fn run<T>(
        &mut self,
        sender: &mut ObjectSender,
        context: &Context,
        purpose: crate::Purpose,
        operation: impl FnOnce(
            &mut ObjectSender,
            &mut dyn FnMut(&[Frame]) -> spiralcore_transfer::Result<Vec<Frame>>,
        ) -> spiralcore_transfer::Result<T>,
    ) -> Result<T> {
        self.check_context(context)?;
        if sender.profile() != self.profile {
            return Err(Error::Protocol);
        }
        let until = self.deadline()?;
        let mut transport_error = None;
        let result = operation(sender, &mut |frames| {
            self.exchange(frames, until, purpose).map_err(|error| {
                transport_error = Some(error);
                spiralcore_transfer::Error::Unknown
            })
        });
        match result {
            Ok(value) if Instant::now() < until => Ok(value),
            Ok(_) => {
                self.cancellation.cancel();
                sender.invalidate_receiver_state();
                Err(Error::Timeout)
            }
            Err(error) => {
                self.cancellation.cancel();
                sender.invalidate_receiver_state();
                Err(transport_error.unwrap_or(Error::Core(error)))
            }
        }
    }
    pub fn send_block(
        &mut self,
        sender: &mut ObjectSender,
        context: &Context,
        bytes: Vec<u8>,
        transfer: Id,
        warm_probe: bool,
    ) -> Result<ObjectReceipt> {
        if bytes.is_empty() || bytes.len() > MAX_BLOCK {
            return Err(Error::Limit);
        }
        self.run(
            sender,
            context,
            crate::Purpose::Application,
            |sender, exchange| {
                sender.send_with_receipt(context, bytes, transfer, warm_probe, exchange)
            },
        )
    }
    pub fn seed_base(
        &mut self,
        sender: &mut ObjectSender,
        context: &Context,
        bytes: Vec<u8>,
        transfer: Id,
    ) -> Result<Stats> {
        if bytes.is_empty() || bytes.len() > 65536 {
            return Err(Error::Limit);
        }
        self.run(
            sender,
            context,
            crate::Purpose::Artifact,
            |sender, exchange| sender.seed(context, bytes, transfer, exchange),
        )
    }
    pub fn seed_schema(
        &mut self,
        sender: &mut ObjectSender,
        context: &Context,
        transfer: Id,
    ) -> Result<Stats> {
        self.run(
            sender,
            context,
            crate::Purpose::Artifact,
            |sender, exchange| sender.seed_schema(context, transfer, exchange),
        )
    }
    pub fn seed_checkpoint(
        &mut self,
        sender: &mut ObjectSender,
        context: &Context,
        transfer: Id,
    ) -> Result<Stats> {
        self.run(
            sender,
            context,
            crate::Purpose::Artifact,
            |sender, exchange| sender.seed_checkpoint(context, transfer, exchange),
        )
    }
    pub fn serve_once<A: Authority>(
        &mut self,
        endpoint: &mut ObjectEndpoint<A>,
        context: &Context,
        now: u64,
    ) -> Result<ServeEvent> {
        self.check_context(context)?;
        if endpoint.profile() != self.profile {
            return Err(Error::Protocol);
        }
        let until = self.deadline()?;
        let result = (|| {
            let (purpose, request) = match self.rx.receive_object(until)? {
                crate::ObjectRecord::Frame(purpose, request) => (purpose, request),
                crate::ObjectRecord::TransportFin => return Ok(ServeEvent::HalfClose),
                crate::ObjectRecord::ApplicationHalfClose => return Err(Error::Protocol),
            };
            if !matches!(request.kind, 1..=6) {
                return Err(Error::Protocol);
            }
            let reply = endpoint.handle(context, &request, now)?;
            self.tx.send_object(&reply, purpose, until)?;
            Ok(ServeEvent::Handled {
                request,
                reply,
                purpose,
            })
        })();
        if result.is_err() {
            self.cancellation.cancel();
        }
        result
    }
    pub fn half_close(&mut self) -> Result<()> {
        self.tx.half_close()
    }
    pub fn receive_terminal(&mut self) -> Result<ServeEvent> {
        let until = self.deadline()?;
        let result = match self.rx.receive_frame(until)? {
            None => Ok(ServeEvent::HalfClose),
            Some(_) => Err(Error::Protocol),
        };
        if result.is_err() {
            self.cancellation.cancel();
        }
        result
    }
}
