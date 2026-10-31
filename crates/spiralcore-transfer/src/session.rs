//! Sender-owned, acknowledged state for repeated transfers.
//!
//! A session is bound to one authenticated application context. The caller must
//! bind that context to its exchange implementation; local readiness is never
//! authorization. All publications still use the existing receiver checks.
//! Failed sends may consume their input. Recovery is explicit and requires the
//! desired bytes and a fresh transfer ID; this module never retries a request.
use crate::{
    client::{self, AdaptiveStats, Stats},
    endpoint::Context,
    protocol::{Frame, Mode},
    schema_selection::{PreparedSelector, WarmSelection},
    shared_transfer::PreparedPrediction,
    Error, Id, Result,
};

/// Conservative per-session admission ceiling, not a process RSS measurement.
/// Applications retaining several sessions must enforce a shared outer budget.
pub const MAX_SESSION_CHARGE: u64 = 512 * 1024 * 1024;
const METADATA_CHARGE: u64 = 4096;
const WARM_TRANSIENT_CHARGE: u64 = 128 * 1024 * 1024;
const RUNTIME_TRANSIENT_CHARGE: u64 = 384 * 1024 * 1024;

// Only this module can construct this value, after the existing client has
// validated publication of these exact owned bytes. No arbitrary hash API.
pub(crate) struct AcknowledgedBase {
    bytes: Vec<u8>,
    id: Id,
}
impl AcknowledgedBase {
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub(crate) fn id(&self) -> Id {
        self.id
    }
}

struct ScopedState {
    namespace: Id,
    base: Option<AcknowledgedBase>,
    ready: bool,
}
impl ScopedState {
    fn new(context: &Context) -> Result<Self> {
        Ok(Self {
            namespace: context.namespace()?,
            base: None,
            ready: false,
        })
    }
    fn check_context(&mut self, context: &Context) -> Result<()> {
        if context.namespace().ok() != Some(self.namespace) {
            self.base = None;
            self.ready = false;
            return Err(Error::Unauthorized);
        }
        Ok(())
    }
    fn promote(&mut self, bytes: Vec<u8>, id: Id) {
        self.base = Some(AcknowledgedBase { bytes, id });
        self.ready = true;
    }
    fn capacity(&self) -> usize {
        self.base.as_ref().map_or(0, |base| base.bytes.capacity())
    }
    fn base_id(&self) -> Option<Id> {
        self.base.as_ref().map(AcknowledgedBase::id)
    }
    fn base_bytes(&self) -> Option<&[u8]> {
        self.base.as_ref().map(AcknowledgedBase::bytes)
    }
}

fn admit(retained: u64, input_capacity: usize, transient: u64) -> Result<()> {
    let peak = retained
        .checked_add(input_capacity as u64)
        .and_then(|n| n.checked_add(transient))
        .ok_or(Error::Limit)?;
    if input_capacity > client::MAX_BATCHED_OBJECT || peak > MAX_SESSION_CHARGE {
        return Err(Error::Limit);
    }
    Ok(())
}

/// Selection/preparation happens before the returned client's elapsed interval.
/// Measure the complete `send` call to include all sender preparation work.
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub transfer: Stats,
    pub selection: WarmSelection,
}

/// One reusable selector and one immutable, last-acknowledged rolling base.
pub struct WarmSession {
    selector: PreparedSelector,
    state: ScopedState,
    schema_ready: bool,
}
impl WarmSession {
    pub fn new(context: &Context, artifact: &[u8]) -> Result<Self> {
        let state = ScopedState::new(context)?;
        let session = Self {
            selector: PreparedSelector::new(artifact)?,
            state,
            schema_ready: false,
        };
        admit(session.retained_charge(), 0, 0)?;
        Ok(session)
    }
    fn check_context(&mut self, context: &Context) -> Result<()> {
        if let Err(error) = self.state.check_context(context) {
            self.schema_ready = false;
            return Err(error);
        }
        Ok(())
    }
    /// Raw seed with no implicit retry or receiver-state claim before its ACK.
    pub fn seed_base<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        bytes: Vec<u8>,
        transfer_id: Id,
        exchange: F,
    ) -> Result<Stats> {
        self.raw(context, bytes, transfer_id, false, exchange)
    }
    /// Explicit Raw identity probe, followed by Raw upload only if absent.
    /// Use a fresh transfer ID; an earlier failed offer may still be recorded.
    pub fn recover<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        bytes: Vec<u8>,
        transfer_id: Id,
        exchange: F,
    ) -> Result<Stats> {
        self.raw(context, bytes, transfer_id, true, exchange)
    }
    fn raw<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        bytes: Vec<u8>,
        transfer_id: Id,
        warm_probe: bool,
        mut exchange: F,
    ) -> Result<Stats> {
        self.check_context(context)?;
        admit(
            self.retained_charge(),
            bytes.capacity(),
            WARM_TRANSIENT_CHARGE,
        )?;
        let mut attempted = false;
        let result =
            client::transfer_batched(&bytes, Mode::Raw, None, transfer_id, warm_probe, |frames| {
                attempted = true;
                exchange(frames)
            });
        match result {
            Ok(stats) => {
                self.state.promote(bytes, stats.object);
                Ok(stats)
            }
            Err(error) => {
                if attempted {
                    self.invalidate_receiver_state();
                }
                Err(error)
            }
        }
    }
    /// Auxiliary schema publication does not replace the rolling object base.
    pub fn seed_schema<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        transfer_id: Id,
        mut exchange: F,
    ) -> Result<Stats> {
        self.check_context(context)?;
        admit(self.retained_charge(), 0, WARM_TRANSIENT_CHARGE)?;
        let mut attempted = false;
        let result = client::transfer_batched(
            self.selector.artifact(),
            Mode::Raw,
            None,
            transfer_id,
            false,
            |frames| {
                attempted = true;
                exchange(frames)
            },
        );
        match result {
            Ok(stats) => {
                self.schema_ready = true;
                Ok(stats)
            }
            Err(error) => {
                if attempted {
                    self.invalidate_receiver_state();
                }
                Err(error)
            }
        }
    }
    /// Select exact bytes using the acknowledged base and reusable contexts.
    /// An unseeded winning schema returns `Missing` before any network call.
    pub fn send<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        bytes: Vec<u8>,
        transfer_id: Id,
        warm_probe: bool,
        mut exchange: F,
    ) -> Result<SessionStats> {
        self.check_context(context)?;
        admit(
            self.retained_charge(),
            bytes.capacity(),
            WARM_TRANSIENT_CHARGE,
        )?;
        let base = self.state.base.as_ref().filter(|_| self.state.ready);
        let prepared =
            self.selector
                .prepare_acknowledged(&bytes, base, self.schema_ready, warm_probe)?;
        if prepared.selection().needs_schema_seed {
            return Err(Error::Missing);
        }
        let selection = prepared.selection().clone();
        let mut attempted = false;
        let result = client::transfer_warm_selection_batched(prepared, transfer_id, |frames| {
            attempted = true;
            exchange(frames)
        });
        match result {
            Ok(transfer) => {
                self.state.promote(bytes, transfer.object);
                Ok(SessionStats {
                    transfer,
                    selection,
                })
            }
            Err(error) => {
                if attempted {
                    self.invalidate_receiver_state();
                }
                Err(error)
            }
        }
    }
    /// Retain local acknowledged bytes, but stop relying on receiver readiness.
    pub fn invalidate_receiver_state(&mut self) {
        self.state.ready = false;
        self.schema_ready = false;
    }
    pub fn base_id(&self) -> Option<Id> {
        self.state.base_id()
    }
    pub fn base_bytes(&self) -> Option<&[u8]> {
        self.state.base_bytes()
    }
    pub fn base_ready(&self) -> bool {
        self.state.ready
    }
    pub fn schema_ready(&self) -> bool {
        self.schema_ready
    }
    /// Owned buffer capacity plus codec-reported allocations; excludes metadata.
    pub fn retained_bytes(&self) -> usize {
        self.selector.retained_bytes() + self.state.capacity()
    }
    /// Conservative retained quota, separate from transient admission and RSS.
    pub fn retained_charge(&self) -> u64 {
        self.selector.retained_charge() + self.state.capacity() as u64 + METADATA_CHARGE
    }
}

/// Reuse one fixed CA/ternary/Kuramoto prediction under its existing codec policy.
/// This never advances the runtime clock or changes the prediction implicitly.
pub struct RuntimeSession {
    prediction: PreparedPrediction,
    state: ScopedState,
    checkpoint_ready: bool,
}
impl RuntimeSession {
    pub fn new(context: &Context, prediction: PreparedPrediction) -> Result<Self> {
        let session = Self {
            state: ScopedState::new(context)?,
            prediction,
            checkpoint_ready: false,
        };
        admit(session.retained_charge(), 0, 0)?;
        Ok(session)
    }
    fn check_context(&mut self, context: &Context) -> Result<()> {
        if let Err(error) = self.state.check_context(context) {
            self.checkpoint_ready = false;
            return Err(error);
        }
        Ok(())
    }
    pub fn seed_checkpoint<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        transfer_id: Id,
        mut exchange: F,
    ) -> Result<Stats> {
        self.check_context(context)?;
        admit(self.retained_charge(), 0, RUNTIME_TRANSIENT_CHARGE)?;
        let mut attempted = false;
        let result = client::transfer_batched(
            self.prediction.checkpoint(),
            Mode::Raw,
            None,
            transfer_id,
            false,
            |frames| {
                attempted = true;
                exchange(frames)
            },
        );
        match result {
            Ok(stats) => {
                self.checkpoint_ready = true;
                Ok(stats)
            }
            Err(error) => {
                if attempted {
                    self.invalidate_receiver_state();
                }
                Err(error)
            }
        }
    }
    pub fn send<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        bytes: Vec<u8>,
        transfer_id: Id,
        warm_probe: bool,
        mut exchange: F,
    ) -> Result<AdaptiveStats> {
        self.check_context(context)?;
        admit(
            self.retained_charge(),
            bytes.capacity(),
            RUNTIME_TRANSIENT_CHARGE,
        )?;
        if !self.checkpoint_ready {
            return Err(Error::Missing);
        }
        let mut attempted = false;
        let result = client::transfer_prepared_runtime_batched(
            &self.prediction,
            &bytes,
            transfer_id,
            warm_probe,
            |frames| {
                attempted = true;
                exchange(frames)
            },
        );
        match result {
            Ok(stats) => {
                self.state.promote(bytes, stats.transfer.object);
                Ok(stats)
            }
            Err(error) => {
                if attempted {
                    self.invalidate_receiver_state();
                }
                Err(error)
            }
        }
    }
    /// Explicit target recovery does not re-establish checkpoint readiness.
    pub fn recover<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        bytes: Vec<u8>,
        transfer_id: Id,
        mut exchange: F,
    ) -> Result<Stats> {
        self.check_context(context)?;
        admit(
            self.retained_charge(),
            bytes.capacity(),
            RUNTIME_TRANSIENT_CHARGE,
        )?;
        let mut attempted = false;
        let result =
            client::transfer_batched(&bytes, Mode::Raw, None, transfer_id, true, |frames| {
                attempted = true;
                exchange(frames)
            });
        match result {
            Ok(stats) => {
                self.state.promote(bytes, stats.object);
                Ok(stats)
            }
            Err(error) => {
                if attempted {
                    self.invalidate_receiver_state();
                }
                Err(error)
            }
        }
    }
    pub fn invalidate_receiver_state(&mut self) {
        self.state.ready = false;
        self.checkpoint_ready = false;
    }
    pub fn base_id(&self) -> Option<Id> {
        self.state.base_id()
    }
    pub fn base_bytes(&self) -> Option<&[u8]> {
        self.state.base_bytes()
    }
    pub fn base_ready(&self) -> bool {
        self.state.ready
    }
    pub fn checkpoint_ready(&self) -> bool {
        self.checkpoint_ready
    }
    /// Owned buffer capacity only; excludes struct/allocator metadata.
    pub fn retained_bytes(&self) -> usize {
        self.prediction.retained_bytes() + self.state.capacity()
    }
    pub fn retained_charge(&self) -> u64 {
        self.retained_bytes() as u64 + METADATA_CHARGE
    }
}
