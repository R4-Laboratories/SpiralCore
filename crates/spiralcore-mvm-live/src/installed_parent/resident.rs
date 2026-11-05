//! One original backend and two cancellation-safe, explicitly driven slots.
use super::{InstalledParentProvider, ParentLaunchOwner, host_agent::RootHostAgent, tenant_tag};
use crate::{
    installed_resources::{ManagedClaim, ManagedReservation, ManagedSnapshot},
    installed_supervisor::InstalledSupervisor,
};
use anyhow::{Result, anyhow, ensure};
use mvm_client::{LaunchOutcome, LaunchRequest, LocalBackend, OwnedRetirement};
use mvm_core::vm_backend::BackendKind;
use std::{
    future::{Future, poll_fn},
    mem::{align_of, size_of},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

/// A local table key, never permission or a retirement proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidentId {
    slot: usize,
    serial: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentStatus {
    Running,
    UnusedRetired,
    PendingCleanup,
    CleanupBlocked,
    Retired,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidentProgress {
    pub id: ResidentId,
    pub state: ResidentStatus,
}

/// Explicitly drive this owner to drain. Dropping a driver await preserves its
/// stored operation. Dropping an incomplete service quarantines all custody.
pub struct InstalledResident {
    live: Option<Box<Live>>,
    supervisor: Option<InstalledSupervisor>,
    closed: bool,
    quarantined: bool,
}
struct Live {
    backend: Arc<LocalBackend>,
    provider: Arc<InstalledParentProvider>,
    helper: Option<RootHostAgent>,
    failed_helper: Option<anyhow::Error>,
    has_issued_vm: bool,
    slots: [Option<Slot>; 2],
    next: usize,
    _storage: ManagedReservation,
}
struct Slot {
    owner: ParentLaunchOwner,
    state: State,
    operation: Option<OwnedOperation>,
    ready: Option<OperationResult>,
    stop_requested: bool,
    error: Option<anyhow::Error>,
    operation_capacity: usize,
    _operation_storage: ManagedReservation,
}
#[allow(
    clippy::large_enum_variant,
    reason = "Fixed two-slot custody uses prepaid inline storage; boxing adds a cleanup-time allocation"
)]
enum State {
    Launching,
    Running(Arc<LaunchOutcome>),
    Unused(mvm_client::UnusedHostLifetimePreparation),
    Pending,
    RetiringOutcome(Arc<LaunchOutcome>),
    RetiringPending(OwnedRetirement),
}
#[allow(
    clippy::large_enum_variant,
    reason = "Fixed two-slot custody uses prepaid inline storage; boxing adds a cleanup-time allocation"
)]
enum OperationResult {
    Launch(std::result::Result<LaunchOutcome, mvm_client::HostLifetimeLaunchFailure>),
    StopOutcome(Arc<LaunchOutcome>, Result<()>),
    StopPending(Result<OwnedRetirement>),
}
struct OwnedOperation {
    // The slot retains storage through launch, stop and positive cleanup.
    future: Pin<Box<dyn Future<Output = OperationResult>>>,
}

// This covers requested Rust storage, not captured heaps, allocator/native
// overhead or process RSS. Bootstrap/config/driver domains remain separate.
fn arc_storage<T>() -> Result<u64> {
    let bytes = size_of::<T>()
        .checked_add(2 * size_of::<usize>())
        .and_then(|n| n.checked_add(align_of::<T>() - 1))
        .ok_or_else(|| anyhow!("resident Arc storage overflow"))?;
    Ok(u64::try_from(bytes)?)
}
fn operation_size<A, B, F>(_: fn(A, B) -> F) -> usize {
    size_of::<F>()
}
async fn stop_outcome(backend: Arc<LocalBackend>, outcome: Arc<LaunchOutcome>) -> OperationResult {
    let result = backend
        .stop_owned(&outcome)
        .await
        .map_err(anyhow::Error::new);
    OperationResult::StopOutcome(outcome, result)
}
async fn stop_pending(backend: Arc<LocalBackend>, grant: Arc<super::VmGrant>) -> OperationResult {
    OperationResult::StopPending(
        backend
            .stop_owned_pending(&grant.binding)
            .await
            .map_err(anyhow::Error::new),
    )
}
impl Slot {
    fn box_operation<F>(&self, make: impl FnOnce() -> F) -> Result<OwnedOperation>
    where
        F: Future<Output = OperationResult> + 'static,
    {
        self._operation_storage.check_retained_identity()?;
        ensure!(
            size_of::<F>() <= self.operation_capacity,
            "original operation storage exceeded"
        );
        Ok(OwnedOperation {
            future: Box::pin(make()),
        })
    }
}

impl InstalledResident {
    pub fn new(supervisor: InstalledSupervisor, backend: LocalBackend) -> Result<Self> {
        let service = Self::prepare(supervisor, backend)?;
        let provider = service.live.as_ref().unwrap().provider.clone();
        mvm_hostd::host_lifetime_parent::install(provider)?;
        Ok(service)
    }

    fn prepare(supervisor: InstalledSupervisor, backend: LocalBackend) -> Result<Self> {
        let provider = InstalledParentProvider::new(&supervisor)?;
        let bytes = u64::try_from(size_of::<Live>())?
            .checked_add(arc_storage::<LocalBackend>()?)
            .and_then(|n| n.checked_add(arc_storage::<InstalledParentProvider>().ok()?))
            .and_then(|n| n.checked_add(2 * arc_storage::<LaunchOutcome>().ok()?))
            .ok_or_else(|| anyhow!("resident storage overflow"))?;
        let storage = provider.shared.pool.reserve(
            tenant_tag("installation-resident"),
            ManagedClaim {
                host_metadata_bytes: bytes,
                ..Default::default()
            },
        )?;
        // Actual metadata admission precedes these allocations. The two outcome
        // Arc bodies are included even while the corresponding slots are empty.
        Ok(Self {
            live: Some(Box::new(Live {
                backend: Arc::new(backend),
                provider: Arc::new(provider),
                helper: None,
                failed_helper: None,
                has_issued_vm: false,
                slots: std::array::from_fn(|_| None),
                next: 0,
                _storage: storage,
            })),
            supervisor: Some(supervisor),
            closed: false,
            quarantined: false,
        })
    }

    pub fn usage(&self) -> Result<ManagedSnapshot> {
        Ok(self
            .supervisor
            .as_ref()
            .ok_or_else(|| anyhow!("resident finished"))?
            .managed_resources()
            .snapshot()?)
    }

    /// Install the configured helper for the canonical LocalBackend tenant.
    /// A missing qualified profile refuses before any helper effects.
    pub fn start_owned_host_agent(&mut self) -> Result<()> {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.start_owned_host_agent_inner()
        }));
        match result {
            Ok(result) => result,
            Err(panic) => {
                self.quarantine();
                std::panic::resume_unwind(panic)
            }
        }
    }

    fn start_owned_host_agent_inner(&mut self) -> Result<()> {
        ensure!(
            !self.closed && !self.quarantined,
            "resident admission closed"
        );
        let live = self
            .live
            .as_mut()
            .ok_or_else(|| anyhow!("resident drained"))?;
        ensure!(
            !live.has_issued_vm
                && live.slots.iter().all(Option::is_none)
                && live.helper.is_none()
                && live.failed_helper.is_none(),
            "helper must precede original VM operations and install once"
        );
        match live.provider.prepare_owned_host_agent("local") {
            Ok(owner) => live.helper = Some(owner),
            Err(error) => match RootHostAgent::from_start_failure(error) {
                Ok(owner) => {
                    live.helper = Some(owner);
                    self.close_admission();
                    return Err(anyhow!(
                        "original helper startup failed; resident retains cleanup"
                    ));
                }
                Err(error) if live.provider.host_agent_unused() => return Err(error),
                Err(error) => {
                    live.failed_helper = Some(error);
                    self.quarantined = true;
                    self.close_admission();
                    return Err(anyhow!(
                        "uncertain helper startup; resident custody retained"
                    ));
                }
            },
        }
        // Store the actual owner before any fallible registrar installation.
        // Failure retains this same backend/helper for explicit shutdown.
        let result = (|| -> Result<()> {
            let registrar = live.helper.as_ref().unwrap().registrar()?;
            live.backend.install_owned_host_agent(registrar)?;
            Ok(())
        })();
        if result.is_err() {
            self.close_admission();
        }
        result
    }

    pub fn begin_launch(&mut self, request: LaunchRequest, tenant: &str) -> Result<ResidentId> {
        self.begin_launch_with_selection(request, tenant, None)
    }

    fn begin_launch_with_selection(
        &mut self,
        request: LaunchRequest,
        tenant: &str,
        mapping: Option<mvm_contract::policy::network_policy::CooperatingPayloadPolicy>,
    ) -> Result<ResidentId> {
        ensure!(!self.closed, "resident admission closed");
        // LocalBackend's canonical admission uses the fixed local tenant.
        ensure!(tenant == "local", "canonical local tenant differs");
        let live = self
            .live
            .as_mut()
            .ok_or_else(|| anyhow!("resident drained"))?;
        let index = live
            .slots
            .iter()
            .position(Option::is_none)
            .ok_or_else(|| anyhow!("resident slots full"))?;
        let backend_kind = if cfg!(target_os = "linux") {
            BackendKind::Firecracker
        } else {
            BackendKind::Hvf
        };
        let prepared =
            live.provider
                .prepare_vm(&request.host_lifetime_intent(), tenant, backend_kind)?;
        // This ordering barrier survives even genuine unused retirement and
        // later failures while constructing the original launch operation.
        live.has_issued_vm = true;
        let id = ResidentId {
            slot: index,
            serial: prepared.binding().grant_serial,
        };
        let backend = live.backend.clone();
        let (operation, owner, storage, capacity) =
            live.own_operation(tenant_tag(tenant), move || {
                let (lifetime, owner) = prepared.begin_launch()?;
                Ok((
                    async move {
                        let outcome = if let Some(mapping) = mapping {
                            backend
                                .try_launch_with_host_lifetime_and_mapping(
                                    request, lifetime, mapping,
                                )
                                .await
                        } else {
                            backend
                                .try_launch_with_host_lifetime(request, lifetime)
                                .await
                        };
                        OperationResult::Launch(outcome)
                    },
                    owner,
                ))
            })?;
        // No canonical poll is possible before original custody occupies a slot.
        live.slots[index] = Some(Slot {
            owner,
            state: State::Launching,
            operation: Some(operation),
            ready: None,
            stop_requested: false,
            error: None,
            operation_capacity: capacity,
            _operation_storage: storage,
        });
        Ok(id)
    }

    pub fn request_stop(&mut self, id: ResidentId) -> Result<()> {
        ensure!(self.live.is_some(), "resident drained");
        self.drive_guarded(|live| {
            live.slot(id)?;
            live.slots[id.slot].as_mut().unwrap().stop_requested = true;
            live.schedule_stop(id.slot)
        })
    }

    /// Borrow diagnostics; do not copy outcomes, authority or error histories.
    pub fn last_error(&self, id: ResidentId) -> Result<Option<&anyhow::Error>> {
        let live = self
            .live
            .as_ref()
            .ok_or_else(|| anyhow!("resident drained"))?;
        Ok(live.slot(id)?.error.as_ref())
    }

    pub async fn drive_once(&mut self) -> Result<ResidentProgress> {
        ensure!(
            !self.quarantined && self.live.is_some(),
            "resident unavailable"
        );
        self.drive_guarded(|live| live.schedule_requested())?;
        poll_fn(|cx| self.drive_guarded(|live| live.poll_once(cx))).await
    }

    fn drive_guarded<T>(&mut self, drive: impl FnOnce(&mut Live) -> T) -> T {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            drive(self.live.as_mut().unwrap())
        })) {
            Ok(result) => result,
            Err(payload) => {
                // Catching this panic outside the service must not leave its
                // possibly advanced canonical future open or pollable again.
                self.quarantine();
                std::panic::resume_unwind(payload)
            }
        }
    }

    fn quarantine(&mut self) {
        self.quarantined = true;
        self.close_admission();
        std::mem::forget((self.live.take(), self.supervisor.take()));
    }

    pub fn close_admission(&mut self) {
        self.closed = true;
        if let Some(live) = &mut self.live {
            live.provider.close_admission();
            for slot in live.slots.iter_mut().flatten() {
                slot.stop_requested = true;
            }
        }
        if let Some(supervisor) = &mut self.supervisor {
            supervisor.close_admission();
        }
    }

    pub fn try_finish(&mut self) -> Result<()> {
        ensure!(!self.quarantined, "resident custody quarantined");
        self.close_admission();
        ensure!(
            self.live
                .as_ref()
                .is_none_or(|live| live.slots.iter().all(Option::is_none)),
            "resident still owns operations or cleanup"
        );
        let shutdown = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some(helper) = self.live.as_mut().and_then(|live| live.helper.as_mut()) {
                helper.shutdown()?;
            }
            Ok::<(), anyhow::Error>(())
        }));
        match shutdown {
            Ok(result) => result?,
            Err(panic) => {
                self.quarantine();
                std::panic::resume_unwind(panic)
            }
        }
        // Weak registration cannot permanently retain these real reservations.
        // Other actual scoped owners still make the SAME global finish refuse.
        drop(self.live.take());
        if let Some(supervisor) = &mut self.supervisor {
            supervisor.try_finish()?;
        }
        self.supervisor.take();
        Ok(())
    }
}
impl Drop for InstalledResident {
    fn drop(&mut self) {
        self.close_admission();
        if self.live.as_ref().is_some_and(|live| {
            live.slots.iter().any(Option::is_some)
                || live.helper.is_some()
                || live.failed_helper.is_some()
        }) {
            // Preserve backend, futures, outcomes, provider and actual resources
            // together. This is bounded fail-closed custody, not background work.
            self.quarantine();
        }
    }
}

impl Live {
    fn slot(&self, id: ResidentId) -> Result<&Slot> {
        let slot = self
            .slots
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or_else(|| anyhow!("resident slot absent"))?;
        ensure!(
            slot.owner.binding().grant_serial == id.serial,
            "resident slot serial differs"
        );
        Ok(slot)
    }

    fn own_operation<F, T>(
        &self,
        tenant: [u8; 32],
        make: impl FnOnce() -> Result<(F, T)>,
    ) -> Result<(OwnedOperation, T, ManagedReservation, usize)>
    where
        F: Future<Output = OperationResult> + 'static,
    {
        let capacity = size_of::<F>()
            .max(operation_size(stop_outcome))
            .max(operation_size(stop_pending))
            .max(1);
        let storage = self.provider.shared.pool.reserve(
            tenant,
            ManagedClaim {
                host_metadata_bytes: u64::try_from(capacity)?,
                ..Default::default()
            },
        )?;
        // F's concrete layout is known without running make. Admission occurs
        // before arming/moving launch custody or boxing its future.
        let (future, other) = make()?;
        Ok((
            OwnedOperation {
                future: Box::pin(future),
            },
            other,
            storage,
            capacity,
        ))
    }

    fn schedule_requested(&mut self) -> Result<()> {
        for index in 0..2 {
            if self.slots[index].as_ref().is_some_and(|slot| {
                slot.stop_requested && slot.operation.is_none() && slot.ready.is_none()
            }) && let Err(error) = self.schedule_stop(index)
            {
                self.slots[index].as_mut().unwrap().error = Some(error);
                return Err(anyhow!("original resident cleanup remains blocked"));
            }
        }
        Ok(())
    }

    fn schedule_stop(&mut self, index: usize) -> Result<()> {
        let slot = self.slots[index]
            .as_ref()
            .ok_or_else(|| anyhow!("resident slot absent"))?;
        if slot.operation.is_some() || slot.ready.is_some() {
            return Ok(());
        }
        let backend = self.backend.clone();
        let operation = match &slot.state {
            State::Running(outcome) => {
                let outcome = outcome.clone();
                slot.box_operation(move || stop_outcome(backend, outcome))?
            }
            State::Pending => {
                let grant = slot.owner.grant.clone();
                slot.box_operation(move || stop_pending(backend, grant))?
            }
            State::Unused(_) | State::RetiringOutcome(_) | State::RetiringPending(_) => {
                return self.complete(index).map(|_| ());
            }
            State::Launching => return Err(anyhow!("launch operation unavailable")),
        };
        self.slots[index].as_mut().unwrap().operation = Some(operation);
        Ok(())
    }

    fn complete(&mut self, index: usize) -> Result<ResidentStatus> {
        let slot = self.slots[index]
            .as_mut()
            .ok_or_else(|| anyhow!("resident slot absent"))?;
        let status = match &mut slot.state {
            State::Unused(certificate) => {
                slot.owner.cancel_unused(certificate)?;
                ResidentStatus::UnusedRetired
            }
            State::RetiringOutcome(outcome) => {
                slot.owner.cleanup_outcome_endpoint(outcome)?;
                slot.owner.finish_retired(outcome)?;
                ResidentStatus::Retired
            }
            State::RetiringPending(receipt) => {
                slot.owner.cleanup_pending_endpoint(receipt)?;
                slot.owner.finish_pending_retired(receipt)?;
                ResidentStatus::Retired
            }
            _ => return Err(anyhow!("original positive cleanup unavailable")),
        };
        // The certificate/outcome/receipt and completed owner drop together.
        self.slots[index].take();
        Ok(status)
    }

    fn poll_once(&mut self, cx: &mut Context<'_>) -> Poll<Result<ResidentProgress>> {
        // Poll both actual operations before returning an event. A ready first
        // slot cannot suppress the second pending operation's waker registration.
        for offset in 0..2 {
            let index = (self.next + offset) % 2;
            if let Some(slot) = &mut self.slots[index]
                && slot.ready.is_none()
                && let Some(operation) = &mut slot.operation
                && let Poll::Ready(result) = operation.future.as_mut().poll(cx)
            {
                slot.ready = Some(result);
                slot.operation.take();
            }
        }
        for offset in 0..2 {
            let index = (self.next + offset) % 2;
            let result = self.slots[index]
                .as_mut()
                .and_then(|slot| slot.ready.take());
            if let Some(result) = result {
                let id = ResidentId {
                    slot: index,
                    serial: self.slots[index]
                        .as_ref()
                        .unwrap()
                        .owner
                        .binding()
                        .grant_serial,
                };
                self.next = (index + 1) % 2;
                let state = self.accept_result(index, result);
                return Poll::Ready(Ok(ResidentProgress { id, state }));
            }
        }
        if self
            .slots
            .iter()
            .flatten()
            .all(|slot| slot.operation.is_none())
        {
            return Poll::Ready(Err(anyhow!(
                "no in-flight resident operation; request cleanup explicitly"
            )));
        }
        Poll::Pending
    }

    fn accept_result(&mut self, index: usize, result: OperationResult) -> ResidentStatus {
        let slot = self.slots[index].as_mut().unwrap();
        match result {
            OperationResult::Launch(Ok(outcome)) => {
                slot.state = State::Running(Arc::new(outcome));
                slot.error = None;
                if slot.stop_requested
                    && let Err(error) = self.schedule_stop(index)
                {
                    self.slots[index].as_mut().unwrap().error = Some(error);
                    return ResidentStatus::CleanupBlocked;
                }
                return ResidentStatus::Running;
            }
            OperationResult::Launch(Err(failure)) => {
                let (error, certificate) = failure.into_parts();
                slot.error = Some(anyhow::Error::new(error));
                let Some(certificate) = certificate else {
                    slot.state = State::Pending;
                    return ResidentStatus::PendingCleanup;
                };
                slot.state = State::Unused(certificate);
            }
            OperationResult::StopOutcome(outcome, result) => {
                if let Err(error) = result {
                    slot.state = State::Running(outcome);
                    slot.error = Some(error);
                    return ResidentStatus::CleanupBlocked;
                }
                slot.state = State::RetiringOutcome(outcome);
            }
            OperationResult::StopPending(result) => match result {
                Ok(receipt) => slot.state = State::RetiringPending(receipt),
                Err(error) => {
                    slot.state = State::Pending;
                    slot.error = Some(error);
                    return ResidentStatus::CleanupBlocked;
                }
            },
        }
        match self.complete(index) {
            Ok(state) => state,
            Err(error) => {
                self.slots[index].as_mut().unwrap().error = Some(error);
                ResidentStatus::CleanupBlocked
            }
        }
    }
}

#[cfg(test)]
#[path = "resident_tests.rs"]
mod tests;

#[cfg(all(test, target_os = "linux"))]
#[path = "native_witness/mod.rs"]
mod native_witness;
