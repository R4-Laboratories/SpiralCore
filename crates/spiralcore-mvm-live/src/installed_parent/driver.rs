//! Keep the original SDK owner reachable across cancellation, native failure and cleanup retry.
use super::*;
use mvm_client::{
    LaunchOutcome, LaunchRequest, LocalBackend, OwnedRetirement, UnusedHostLifetimePreparation,
};
use mvm_hostd::run::DirectTcpPolicy;

#[must_use = "retain the launch until retirement or explicit recovery"]
pub struct CanonicalLaunch<'a> {
    backend: &'a LocalBackend,
    state: State,
    endpoint: Option<mvm_client::EndpointRetirement>,
}

enum State {
    Prepared(PreparedParentVm),
    Active(Box<Active>),
    Retired,
}

struct Active {
    owner: ParentLaunchOwner,
    evidence: Evidence,
}

enum Evidence {
    Invoked,
    Unused(UnusedHostLifetimePreparation),
    Running(Box<LaunchOutcome>),
    Pending(OwnedRetirement),
}

impl PreparedParentVm {
    /// Tie the original Firecracker backend to this single installation grant.
    pub fn canonical_driver(self, backend: &LocalBackend) -> CanonicalLaunch<'_> {
        CanonicalLaunch {
            backend,
            state: State::Prepared(self),
            endpoint: None,
        }
    }
}

impl CanonicalLaunch<'_> {
    /// Dispatch once. On an interrupted or refused call, explicit retirement
    /// checks the same SDK instance and still-held original parent grant.
    pub async fn start(&mut self, request: LaunchRequest, policy: DirectTcpPolicy) -> Result<()> {
        let State::Prepared(prepared) = &self.state else {
            anyhow::bail!("original startup has already advanced");
        };
        let intent = request.host_lifetime_intent();
        let binding = prepared.binding();
        ensure!(
            intent.mode == mvm_client::LifecycleMode::Transient
                && intent.name.as_deref() == Some(binding.vm_name.as_str())
                && intent.cpus == binding.cpus
                && intent.memory_mib == binding.memory_mib
                && intent
                    .backend
                    .as_deref()
                    .is_none_or(|name| name == "firecracker")
                && binding.backend == BackendKind::Firecracker,
            "request differs from prepared Firecracker owner"
        );
        let kernel = prepared.original_kernel()?;
        // All remaining errors before begin_launch returns are pre-invocation.
        let State::Prepared(prepared) = std::mem::replace(&mut self.state, State::Retired) else {
            unreachable!("checked exclusive preparation");
        };
        let (lifetime, owner) = prepared.begin_launch()?;
        self.state = State::Active(Box::new(Active {
            owner,
            evidence: Evidence::Invoked,
        }));
        let result = self
            .backend
            .try_launch_with_canonical_direct_policy(request, lifetime, policy, kernel)
            .await;
        let State::Active(active) = &mut self.state else {
            unreachable!("start holds the exclusive original owner");
        };
        match result {
            Ok(outcome) => {
                active.evidence = Evidence::Running(Box::new(outcome));
                Ok(())
            }
            Err(failure) => {
                let (error, unused) = failure.into_parts();
                if let Some(unused) = unused {
                    active.evidence = Evidence::Unused(unused);
                }
                Err(error.into())
            }
        }
    }

    pub fn outcome(&self) -> Result<&LaunchOutcome> {
        match &self.state {
            State::Active(active) if let Evidence::Running(outcome) = &active.evidence => {
                Ok(outcome)
            }
            _ => anyhow::bail!("original admitted SDK outcome unavailable"),
        }
    }

    /// The original admitted session and the guest public key observed at boot.
    pub fn flowmux_guest(&self) -> Result<ObservedFlowMuxGuest> {
        let outcome = self.outcome()?;
        let State::Active(active) = &self.state else {
            unreachable!("outcome requires the active original owner");
        };
        let endpoint = active
            .owner
            .grant
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?
            .endpoint_owner
            .clone()
            .ok_or_else(|| anyhow!("original endpoint missing"))?;
        endpoint.observed_guest(&outcome.admitted.plan_id().0)
    }

    /// Invoke the boot-validated workload once on the original admitted guest.
    pub fn run_entrypoint(
        &self,
        until: std::time::Instant,
        observer: &mut dyn mvm_client::entrypoint::CallObserver,
    ) -> Result<mvm_client::entrypoint::CallOutcome> {
        Ok(self
            .backend
            .run_owned_entrypoint(self.outcome()?, until, observer)?)
    }

    /// Read this boot's original workload exit report within one absolute deadline.
    pub fn wait_until(
        &self,
        until: std::time::Instant,
    ) -> Result<mvm_core::vm_backend::VmExitStatus> {
        Ok(self.backend.wait_owned_until(self.outcome()?, until)?)
    }

    /// Retirement may be retried after an original stop, archive or disk I/O failure.
    pub async fn retire(&mut self) -> Result<()> {
        let State::Active(active) = &mut self.state else {
            // An unpolled prepared launch has not transferred its grant to the SDK.
            self.state = State::Retired;
            return Ok(());
        };
        let Active { owner, evidence } = active.as_mut();
        if matches!(evidence, Evidence::Invoked) {
            let original = self.backend.stop_owned_pending(owner.binding()).await?;
            *evidence = Evidence::Pending(original);
        }
        match evidence {
            Evidence::Unused(unused) => owner.cancel_unused(unused)?,
            Evidence::Running(outcome) => {
                self.backend.stop_owned(outcome).await?;
                settle_endpoint(owner, &mut self.endpoint, || {
                    Ok(outcome.take_endpoint_retirement()?)
                })?;
                owner.finish_retired(outcome)?;
            }
            Evidence::Pending(receipt) => {
                settle_endpoint(owner, &mut self.endpoint, || {
                    Ok(receipt.take_endpoint_retirement())
                })?;
                owner.finish_pending_retired(receipt)?;
            }
            Evidence::Invoked => unreachable!("pending retirement was retained"),
        }
        self.state = State::Retired;
        Ok(())
    }
}

fn settle_endpoint(
    owner: &mut ParentLaunchOwner,
    pending: &mut Option<mvm_client::EndpointRetirement>,
    take: impl FnOnce() -> Result<Option<mvm_client::EndpointRetirement>>,
) -> Result<()> {
    if owner.endpoint_retirement.is_some() {
        return owner.retry_endpoint_cleanup();
    }
    if pending.is_none() {
        *pending = take()?;
    }
    if pending.is_some() {
        owner.retire_endpoint(pending)?;
    }
    Ok(())
}
