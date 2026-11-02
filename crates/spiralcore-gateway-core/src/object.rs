//! Shared sender facade. A scoped namespace is not a substitute for authentication.
use spiralcore_transfer::{
    client::{self, Stats},
    endpoint::Context,
    protocol::{Frame, Mode},
    schema_selection::WarmSelection,
    selection::Selection,
    session::{CombinedSelection, CombinedSession, RuntimeSession, SelectionPolicy, WarmSession},
    shared_transfer::PreparedPrediction,
    zstd_whole::PreparedEncoder,
    Error, Id, Result,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectMode {
    Raw,
    Whole,
    Warm,
    Runtime,
    Combined,
}
/// The underlying profile's actual receipt, retaining its HTTP-body cost model.
/// Combined compares its configured candidates; it does not cover every codec.
#[derive(Debug, Clone)]
pub enum ObjectSelection {
    Fixed,
    Warm(WarmSelection),
    Runtime(Selection),
    Combined(CombinedSelection),
}
#[derive(Debug, Clone)]
pub struct ObjectReceipt {
    pub transfer: Stats,
    pub selection: ObjectSelection,
}
enum Prepared {
    Raw,
    Whole(PreparedEncoder),
    Warm(Box<WarmSession>),
    Runtime(Box<RuntimeSession>),
    Combined(Box<CombinedSession>),
}
pub struct ObjectSender {
    namespace: Id,
    prepared: Option<Prepared>,
}
impl ObjectSender {
    /// Runtime/Combined require their constructors with an explicit prediction.
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
    /// Moves one fixed prediction into the existing scoped runtime session.
    /// Prediction construction/replay is a separate caller-paid setup cost.
    pub fn with_runtime(context: &Context, prediction: PreparedPrediction) -> Result<Self> {
        Ok(Self {
            namespace: context.namespace()?,
            prepared: Some(Prepared::Runtime(Box::new(RuntimeSession::new(
                context, prediction,
            )?))),
        })
    }
    /// One prediction and one rolling acknowledged base share prepared codecs.
    /// Checkpoint/schema readiness is established by explicit publication.
    pub fn with_combined(
        context: &Context,
        prediction: PreparedPrediction,
        schema: &[u8],
    ) -> Result<Self> {
        Self::with_combined_policy(context, prediction, schema, SelectionPolicy::Exhaustive)
    }
    /// Explicit sender eligibility policy. CpuConscious may miss a smaller
    /// schema encoding; exact reconstruction and receiver authorization remain required.
    pub fn with_combined_policy(
        context: &Context,
        prediction: PreparedPrediction,
        schema: &[u8],
        policy: SelectionPolicy,
    ) -> Result<Self> {
        Ok(Self {
            namespace: context.namespace()?,
            prepared: Some(Prepared::Combined(Box::new(
                CombinedSession::new_with_policy(context, prediction, schema, policy)?,
            ))),
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
            Prepared::Runtime(_) | Prepared::Combined(_) => Err(Error::Unsupported),
            _ => client::transfer_batched(&bytes, Mode::Raw, None, transfer, false, exchange),
        }
    }
    /// Explicit verified checkpoint publication; no target send seeds it implicitly.
    pub fn seed_checkpoint<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        transfer: Id,
        exchange: F,
    ) -> Result<Stats> {
        match self.state(context)? {
            Prepared::Runtime(sender) => sender.seed_checkpoint(context, transfer, exchange),
            Prepared::Combined(sender) => sender.seed_checkpoint(context, transfer, exchange),
            _ => Err(Error::Unsupported),
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
            Prepared::Combined(sender) => sender.seed_schema(context, transfer, exchange),
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
        self.send_with_receipt(context, bytes, transfer, warm_probe, exchange)
            .map(|receipt| receipt.transfer)
    }
    /// Measure this whole method for preparation plus transfer cost. Candidate
    /// costs describe the original HTTP-body model, not the caller's transport.
    pub fn send_with_receipt<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        bytes: Vec<u8>,
        transfer: Id,
        warm_probe: bool,
        exchange: F,
    ) -> Result<ObjectReceipt> {
        let state = self.state(context)?;
        if bytes.len() > client::MAX_BATCHED_OBJECT {
            return Err(Error::Limit);
        }
        let (transfer, selection) = match state {
            Prepared::Raw => (
                client::transfer_batched(&bytes, Mode::Raw, None, transfer, warm_probe, exchange)?,
                ObjectSelection::Fixed,
            ),
            Prepared::Whole(sender) => (
                client::transfer_prepared_zstd_whole_batched(
                    sender, &bytes, transfer, warm_probe, exchange,
                )?,
                ObjectSelection::Fixed,
            ),
            Prepared::Warm(sender) => {
                let receipt = sender.send(context, bytes, transfer, warm_probe, exchange)?;
                (receipt.transfer, ObjectSelection::Warm(receipt.selection))
            }
            Prepared::Runtime(sender) => {
                let receipt = sender.send(context, bytes, transfer, warm_probe, exchange)?;
                (
                    receipt.transfer,
                    ObjectSelection::Runtime(receipt.selection),
                )
            }
            Prepared::Combined(sender) => {
                let receipt = sender.send(context, bytes, transfer, warm_probe, exchange)?;
                (
                    receipt.transfer,
                    ObjectSelection::Combined(receipt.selection),
                )
            }
        };
        Ok(ObjectReceipt {
            transfer,
            selection,
        })
    }
    /// Retain prepared local state but discard receiver-readiness assumptions.
    /// A changed context still permanently destroys this facade's prepared state.
    pub fn invalidate_receiver_state(&mut self) {
        match self.prepared.as_mut() {
            Some(Prepared::Warm(sender)) => sender.invalidate_receiver_state(),
            Some(Prepared::Runtime(sender)) => sender.invalidate_receiver_state(),
            Some(Prepared::Combined(sender)) => sender.invalidate_receiver_state(),
            _ => {}
        }
    }
    /// Existing profile's conservative retained quota, not process RSS or peak
    /// send work. Callers with multiple senders must enforce a shared budget.
    pub fn retained_charge(&self) -> u64 {
        match self.prepared.as_ref() {
            Some(Prepared::Whole(sender)) => sender.retained_charge(),
            Some(Prepared::Warm(sender)) => sender.retained_charge(),
            Some(Prepared::Runtime(sender)) => sender.retained_charge(),
            Some(Prepared::Combined(sender)) => sender.retained_charge(),
            _ => 0,
        }
    }
}
