//! Shared host accounting and clock ownership for synchronous local dispatch.
//!
//! Create one context for a broker and clone it for every endpoint session.
//! Each dispatch samples time only after acquiring the shared gate, then holds
//! that gate through the local service action. A shared `Instant` alone is not
//! sufficient: workers can sample in one order and reach budget accounting in
//! another, violating the service's nondecreasing monotonic-clock contract.
//!
//! Do not perform socket I/O, wait for another dispatch, or call `with_time`
//! recursively inside an action. Release dispatch before transport waits and
//! reacquire it for the next local phase. A sampled time must not be saved for
//! use in later service operations outside the action. This context is local
//! scheduling/accounting, not a replacement for canonical authentication.
use crate::service::{HostBudget, ServiceError, ServiceTime};
use std::{
    sync::{Arc, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

struct Shared {
    budget: HostBudget,
    origin: Instant,
    dispatch: Mutex<()>,
}

/// Clones share one budget, monotonic origin and synchronous dispatch gate.
#[derive(Clone)]
pub struct BrokerContext {
    shared: Arc<Shared>,
}
impl BrokerContext {
    /// Start the broker with its newly provisioned host budget. All sessions
    /// using that budget must use clones of this context, rather than creating
    /// additional independent contexts or dispatching directly around it.
    pub fn new(budget: HostBudget) -> Self {
        Self {
            shared: Arc::new(Shared {
                budget,
                origin: Instant::now(),
                dispatch: Mutex::new(()),
            }),
        }
    }

    /// Serialize one synchronous service action and sample its trusted clocks
    /// after admission to dispatch. Application errors are returned unchanged;
    /// poisoned dispatch state and invalid clock conversions fail closed.
    pub fn with_time<T, E: From<ServiceError>>(
        &self,
        action: impl FnOnce(&HostBudget, ServiceTime) -> Result<T, E>,
    ) -> Result<T, E> {
        let _dispatch = self
            .shared
            .dispatch
            .lock()
            .map_err(|_| E::from(ServiceError::State))?;
        let unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| E::from(ServiceError::Clock))?
            .as_millis();
        let now = ServiceTime {
            unix_ms: i64::try_from(unix_ms).map_err(|_| E::from(ServiceError::Clock))?,
            monotonic_ms: u64::try_from(self.shared.origin.elapsed().as_millis())
                .map_err(|_| E::from(ServiceError::Clock))?,
        };
        action(&self.shared.budget, now)
    }
}
