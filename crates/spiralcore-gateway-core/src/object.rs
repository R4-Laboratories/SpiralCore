//! Shared sender facade. A scoped namespace is not a substitute for authentication.
use spiralcore_transfer::{
    client::{self, Stats},
    endpoint::Context,
    protocol::{Frame, Mode},
    session::WarmSession,
    zstd_whole::PreparedEncoder,
    Error, Id, Result,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectMode {
    Raw,
    Whole,
    Warm,
}
enum Prepared {
    Raw,
    Whole(PreparedEncoder),
    Warm(Box<WarmSession>),
}
pub struct ObjectSender {
    namespace: Id,
    prepared: Option<Prepared>,
}
impl ObjectSender {
    pub fn new(mode: ObjectMode, context: &Context, schema: Option<&[u8]>) -> Result<Self> {
        let namespace = context.namespace()?;
        let prepared = match (mode, schema) {
            (ObjectMode::Raw, None) => Prepared::Raw,
            (ObjectMode::Whole, None) => Prepared::Whole(PreparedEncoder::new()?),
            (ObjectMode::Warm, Some(schema)) => {
                Prepared::Warm(Box::new(WarmSession::new(context, schema)?))
            }
            _ => return Err(Error::Malformed),
        };
        Ok(Self {
            namespace,
            prepared: Some(prepared),
        })
    }
    fn state(&mut self, context: &Context) -> Result<&mut Prepared> {
        if context.namespace().ok() != Some(self.namespace) {
            self.prepared = None;
        }
        self.prepared.as_mut().ok_or(Error::Unauthorized)
    }
    pub fn seed<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        bytes: Vec<u8>,
        transfer: Id,
        exchange: F,
    ) -> Result<Stats> {
        match self.state(context)? {
            Prepared::Warm(sender) => sender.seed_base(context, bytes, transfer, exchange),
            _ => client::transfer_batched(&bytes, Mode::Raw, None, transfer, false, exchange),
        }
    }
    pub fn seed_schema<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        transfer: Id,
        exchange: F,
    ) -> Result<Stats> {
        match self.state(context)? {
            Prepared::Warm(sender) => sender.seed_schema(context, transfer, exchange),
            _ => Err(Error::Unsupported),
        }
    }
    pub fn send<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        bytes: Vec<u8>,
        transfer: Id,
        warm_probe: bool,
        exchange: F,
    ) -> Result<Stats> {
        let state = self.state(context)?;
        if bytes.len() > client::MAX_BATCHED_OBJECT {
            return Err(Error::Limit);
        }
        match state {
            Prepared::Raw => {
                client::transfer_batched(&bytes, Mode::Raw, None, transfer, warm_probe, exchange)
            }
            Prepared::Whole(sender) => client::transfer_prepared_zstd_whole_batched(
                sender, &bytes, transfer, warm_probe, exchange,
            ),
            Prepared::Warm(sender) => sender
                .send(context, bytes, transfer, warm_probe, exchange)
                .map(|s| s.transfer),
        }
    }
}
