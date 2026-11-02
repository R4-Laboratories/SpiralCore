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
    schema_selection::{CandidateCost, PreparedSelector, WarmSelection},
    selection,
    shared_transfer::PreparedPrediction,
    Error, Id, Result, MAX_CHUNK,
};

pub use crate::schema_selection::CombinedSelection;

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
    /// Exact acknowledged repeats always probe Raw Begin alone, irrespective of
    /// `warm_probe`, then upload Raw only on a receiver miss. Their selection
    /// receipt reports only that Raw miss cost, not an unevaluated codec minimum.
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
        let mut attempted = false;
        let (result, selection) = if let Some(base) = base.filter(|b| b.bytes() == bytes.as_slice())
        {
            let cost = selection::cost(
                Mode::Raw,
                bytes.chunks(MAX_CHUNK).map(|chunk| chunk.len() + 5),
                true,
            )?;
            let selection = WarmSelection {
                candidates: vec![CandidateCost {
                    mode: Mode::Raw,
                    target_http_body_bytes: cost,
                    setup_http_body_bytes: 0,
                    total_http_body_bytes: cost,
                }],
                pruned: Vec::new(),
                selected_mode: Mode::Raw,
                needs_schema_seed: false,
            };
            let result =
                client::transfer_acknowledged_repeat_batched(base, transfer_id, |frames| {
                    attempted = true;
                    exchange(frames)
                });
            (result, selection)
        } else {
            let prepared =
                self.selector
                    .prepare_acknowledged(&bytes, base, self.schema_ready, warm_probe)?;
            if prepared.selection().needs_schema_seed {
                return Err(Error::Missing);
            }
            let selection = prepared.selection().clone();
            let result = client::transfer_warm_selection_batched(prepared, transfer_id, |frames| {
                attempted = true;
                exchange(frames)
            });
            (result, selection)
        };
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

#[derive(Debug, Clone)]
pub struct CombinedStats {
    pub transfer: Stats,
    pub selection: CombinedSelection,
}

struct CombinedPrepared {
    prediction: PreparedPrediction,
    selector: PreparedSelector,
}

/// One fixed prediction plus one last-acknowledged rolling object. Auxiliary
/// seeds are explicit; the first target uses normal byte-first selection.
/// Namespace mismatch permanently discards the prepared state. No caller-supplied
/// known-hash interface can bypass equality or the receiver's current ACK.
pub struct CombinedSession {
    prepared: Option<CombinedPrepared>,
    state: ScopedState,
    checkpoint_ready: bool,
    schema_ready: bool,
}
impl CombinedSession {
    pub fn new(context: &Context, prediction: PreparedPrediction, artifact: &[u8]) -> Result<Self> {
        let session = Self {
            prepared: Some(CombinedPrepared {
                prediction,
                selector: PreparedSelector::new(artifact)?,
            }),
            state: ScopedState::new(context)?,
            checkpoint_ready: false,
            schema_ready: false,
        };
        admit(session.retained_charge(), 0, 0)?;
        Ok(session)
    }

    fn check_context(&mut self, context: &Context) -> Result<()> {
        if self.prepared.is_none() {
            return Err(Error::Unauthorized);
        }
        if let Err(error) = self.state.check_context(context) {
            self.prepared = None;
            self.checkpoint_ready = false;
            self.schema_ready = false;
            return Err(error);
        }
        Ok(())
    }

    /// Does not replace the rolling target, including on a duplicate seed.
    pub fn seed_checkpoint<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        transfer_id: Id,
        exchange: F,
    ) -> Result<Stats> {
        self.seed_auxiliary(context, transfer_id, true, exchange)
    }

    /// An unseeded schema is excluded from selection, never seeded implicitly.
    pub fn seed_schema<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        transfer_id: Id,
        exchange: F,
    ) -> Result<Stats> {
        self.seed_auxiliary(context, transfer_id, false, exchange)
    }

    fn seed_auxiliary<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        transfer_id: Id,
        checkpoint: bool,
        mut exchange: F,
    ) -> Result<Stats> {
        self.check_context(context)?;
        admit(self.retained_charge(), 0, WARM_TRANSIENT_CHARGE)?;
        let prepared = self.prepared.as_ref().ok_or(Error::Unauthorized)?;
        let bytes = if checkpoint {
            prepared.prediction.checkpoint()
        } else {
            prepared.selector.artifact()
        };
        let mut attempted = false;
        let result =
            client::transfer_batched(bytes, Mode::Raw, None, transfer_id, false, |frames| {
                attempted = true;
                exchange(frames)
            });
        match result {
            Ok(stats) => {
                if checkpoint {
                    self.checkpoint_ready = true;
                } else {
                    self.schema_ready = true;
                }
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

    /// Selection costs retain the existing successful-miss HTTP-body model.
    /// Measure this whole call for candidate preparation/hash/transfer work.
    /// Exact repeats skip codecs and hashing before a current Raw identity probe;
    /// remote eviction uploads Raw under that same offer, without reseeding.
    pub fn send<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &mut self,
        context: &Context,
        bytes: Vec<u8>,
        transfer_id: Id,
        warm_probe: bool,
        mut exchange: F,
    ) -> Result<CombinedStats> {
        self.check_context(context)?;
        // Whole+Schema contexts are already charged as retained (256 MiB plus
        // schema tables), unlike RuntimeSession's fresh-context scratch budget.
        // No replay allocation occurs here. Sequential delta preparation holds
        // at most two <=16 MiB recipes plus the incumbent. The larger schema
        // path holds its <=16.32 MiB intermediate, bounded compressed output,
        // winning framed copy and old incumbent: <66 MiB plus small metadata.
        // Existing 128 MiB scratch covers those temporaries and bounded batched
        // client copies. Retained prediction/base and input capacity are charged
        // separately, yielding <433 MiB at the 16 MiB ceiling. Transport and
        // concurrent sessions still require the caller's shared outer admission.
        admit(
            self.retained_charge(),
            bytes.capacity(),
            WARM_TRANSIENT_CHARGE,
        )?;
        let base = self.state.base.as_ref().filter(|_| self.state.ready);
        let mut attempted = false;
        let (result, selection) = if let Some(base) = base.filter(|b| b.bytes() == bytes.as_slice())
        {
            let cost = selection::cost(
                Mode::Raw,
                bytes.chunks(MAX_CHUNK).map(|c| c.len() + 5),
                true,
            )?;
            let selection = CombinedSelection {
                candidates: vec![CandidateCost {
                    mode: Mode::Raw,
                    target_http_body_bytes: cost,
                    setup_http_body_bytes: 0,
                    total_http_body_bytes: cost,
                }],
                pruned: Vec::new(),
                selected_mode: Mode::Raw,
                exact_repeat_probe: true,
                checkpoint_ready: self.checkpoint_ready,
                schema_ready: self.schema_ready,
            };
            let result =
                client::transfer_acknowledged_repeat_batched(base, transfer_id, |frames| {
                    attempted = true;
                    exchange(frames)
                });
            (result, selection)
        } else {
            if !self.checkpoint_ready {
                return Err(Error::Missing);
            }
            let prepared = self.prepared.as_mut().ok_or(Error::Unauthorized)?;
            let (transfer, selection) = prepared.selector.prepare_combined_acknowledged(
                &bytes,
                base,
                &prepared.prediction,
                self.schema_ready,
                warm_probe,
            )?;
            let result = client::transfer_warm_selection_batched(transfer, transfer_id, |frames| {
                attempted = true;
                exchange(frames)
            });
            (result, selection)
        };
        match result {
            Ok(transfer) => {
                self.state.promote(bytes, transfer.object);
                Ok(CombinedStats {
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

    pub fn invalidate_receiver_state(&mut self) {
        self.state.ready = false;
        self.checkpoint_ready = false;
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
    pub fn checkpoint_ready(&self) -> bool {
        self.checkpoint_ready
    }
    pub fn schema_ready(&self) -> bool {
        self.schema_ready
    }
    pub fn retained_bytes(&self) -> usize {
        self.state.capacity()
            + self.prepared.as_ref().map_or(0, |p| {
                p.prediction.retained_bytes() + p.selector.retained_bytes()
            })
    }
    pub fn retained_charge(&self) -> u64 {
        self.state.capacity() as u64
            + METADATA_CHARGE
            + self.prepared.as_ref().map_or(0, |p| {
                p.prediction.retained_bytes() as u64 + p.selector.retained_charge()
            })
    }
}
