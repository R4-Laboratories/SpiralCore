#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::AtomicBool};
    use std::time::{Duration, Instant};
    #[test]
    fn release_without_progress_wakes_an_enrolled_waiter() {
        let owner = Arc::new(ReleaseMutex::new(7));
        let stop = AtomicBool::new(false);
        let guard = owner.try_acquire().unwrap().unwrap();
        std::thread::scope(|scope| {
            let waiter = scope.spawn(|| {
                owner
                    .acquire(Instant::now() + Duration::from_secs(2), &stop)
                    .map(|g| *g)
            });
            owner.wait_for_enrollment();
            drop(guard);
            assert_eq!(waiter.join().unwrap(), Ok(7));
        });
    }
    #[test]
    fn poisoned_gate_refuses_even_uncontended_data() {
        let owner = ReleaseMutex::new(1);
        let _ = std::panic::catch_unwind(|| {
            let _gate = owner.gate.mutex.lock().unwrap();
            panic!("poison gate");
        });
        assert!(matches!(owner.try_acquire(), Err(AcquireError::Poisoned)));
    }
    #[test]
    fn cancellation_wakes_waiter_while_original_owner_remains_held() {
        let owner = ReleaseMutex::new(1);
        let stop = AtomicBool::new(false);
        let held = owner.try_acquire().unwrap().unwrap();
        std::thread::scope(|scope| {
            let waiter = scope.spawn(|| {
                owner
                    .acquire(Instant::now() + Duration::from_secs(2), &stop)
                    .err()
            });
            owner.wait_for_enrollment();
            stop.store(true, Ordering::Release);
            owner.gate.notify();
            assert_eq!(waiter.join().unwrap(), Some(AcquireError::Stopped));
            drop(held);
        });
    }
    #[test]
    fn cancellation_before_enrollment_cannot_strand_a_waiter() {
        let owner = ReleaseMutex::new(1);
        let stop = AtomicBool::new(false);
        let held = owner.try_acquire().unwrap().unwrap();
        let gate = owner.gate.mutex.lock().unwrap();
        let started = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            let waiter = scope.spawn(|| {
                started.wait();
                owner
                    .acquire(Instant::now() + Duration::from_secs(2), &stop)
                    .err()
            });
            started.wait();
            stop.store(true, Ordering::Release);
            drop(gate);
            owner.gate.notify();
            assert_eq!(waiter.join().unwrap(), Some(AcquireError::Stopped));
            drop(held);
        });
    }
    #[test]
    fn spurious_notification_preserves_predicate_and_multiple_waiters() {
        let owner = ReleaseMutex::new(0);
        let stop = AtomicBool::new(false);
        let held = owner.try_acquire().unwrap().unwrap();
        std::thread::scope(|scope| {
            let waiters: Vec<_> = (0..3)
                .map(|_| {
                    scope.spawn(|| {
                        let mut g = owner
                            .acquire(Instant::now() + Duration::from_secs(2), &stop)
                            .unwrap();
                        *g += 1;
                        *g
                    })
                })
                .collect();
            owner.wait_for_enrollments(3);
            owner.gate.notify();
            owner.wait_for_enrollments(6);
            drop(held);
            let mut values: Vec<_> = waiters.into_iter().map(|w| w.join().unwrap()).collect();
            values.sort();
            assert_eq!(values, [1, 2, 3]);
        });
    }
    #[test]
    fn original_deadline_refuses_held_and_later_uncontended_data() {
        let owner = ReleaseMutex::new(1);
        let stop = AtomicBool::new(false);
        let held = owner.try_acquire().unwrap().unwrap();
        let until = Instant::now() + Duration::from_millis(20);
        assert_eq!(
            owner.acquire(until, &stop).err(),
            Some(AcquireError::Timeout)
        );
        drop(held);
        assert_eq!(
            owner.acquire(until, &stop).err(),
            Some(AcquireError::Timeout)
        );
    }
    #[test]
    fn original_data_poison_is_never_recovered_and_drop_notifies_during_unwind() {
        let owner = ReleaseMutex::new(1);
        let stop = AtomicBool::new(false);
        let held = owner.try_acquire().unwrap().unwrap();
        std::thread::scope(|scope| {
            let waiter = scope.spawn(|| {
                owner
                    .acquire(Instant::now() + Duration::from_secs(2), &stop)
                    .err()
            });
            owner.wait_for_enrollment();
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let _held = held;
                panic!("poison data");
            }));
            assert_eq!(waiter.join().unwrap(), Some(AcquireError::Poisoned));
        });
        assert!(!owner.gate.mutex.is_poisoned());
        assert_eq!(
            owner
                .acquire(Instant::now() + Duration::from_secs(1), &stop)
                .err(),
            Some(AcquireError::Poisoned)
        );
    }

    #[test]
    fn drop_does_not_panic_when_gate_has_been_poisoned() {
        let owner = ReleaseMutex::new(1);
        let held = owner.try_acquire().unwrap().unwrap();
        let _ = std::panic::catch_unwind(|| {
            let _gate = owner.gate.mutex.lock().unwrap();
            panic!("poison gate");
        });
        drop(held);
        assert!(matches!(owner.try_acquire(), Err(AcquireError::Poisoned)));
    }
    #[test]
    fn release_before_enrollment_is_observed_without_a_second_notification() {
        let owner = ReleaseMutex::new(1);
        let stop = AtomicBool::new(false);
        std::thread::scope(|scope| {
            let (held_tx, held_rx) = std::sync::mpsc::sync_channel(1);
            let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
            let owner_ref = &owner;
            let holder = scope.spawn(move || {
                let held = owner_ref.try_acquire().unwrap().unwrap();
                held_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                drop(held);
            });
            held_rx.recv().unwrap();
            let gate = owner.gate.mutex.lock().unwrap();
            release_tx.send(()).unwrap();
            // Observe the original data unlock while its Drop is blocked on the gate.
            let until = Instant::now() + Duration::from_secs(2);
            loop {
                if let Ok(g) = owner.data.try_lock() {
                    drop(g);
                    break;
                }
                assert!(Instant::now() < until);
                std::thread::yield_now();
            }
            let stop_ref = &stop;
            let waiter = scope.spawn(move || owner_ref.acquire(until, stop_ref).map(|g| *g));
            drop(gate);
            assert_eq!(waiter.join().unwrap(), Ok(1));
            holder.join().unwrap();
        });
    }
}

use std::{
    ops::{Deref, DerefMut},
    sync::{
        Arc, Condvar, Mutex, MutexGuard, TryLockError,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AcquireError {
    Stopped,
    Timeout,
    Poisoned,
}

pub(crate) struct Gate {
    mutex: Mutex<()>,
    changed: Condvar,
}
impl Gate {
    pub(crate) fn notify(&self) {
        // Notification during unwind must neither panic nor authorize poisoned state.
        let _gate = self.mutex.lock().unwrap_or_else(|error| error.into_inner());
        self.changed.notify_all();
    }
}
#[cfg(feature = "witness-timing")]
use crate::timing::{LockKind, LockOutcome, Metric, TimingObserver};
pub(crate) struct ReleaseMutex<T> {
    data: Mutex<T>,
    gate: Arc<Gate>,
    #[cfg(feature = "witness-timing")]
    timing: Option<(TimingObserver, LockKind)>,
    #[cfg(test)]
    enrollments: std::sync::atomic::AtomicUsize,
}
pub(crate) struct Guard<'a, T> {
    data: Option<MutexGuard<'a, T>>,
    gate: &'a Gate,
}
impl<T> Deref for Guard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data.as_deref().expect("live original guard")
    }
}
impl<T> DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data.as_deref_mut().expect("live original guard")
    }
}
impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        drop(self.data.take());
        self.gate.notify();
    }
}
impl<T> ReleaseMutex<T> {
    pub(crate) fn new(value: T) -> Self {
        Self {
            data: Mutex::new(value),
            #[cfg(feature = "witness-timing")]
            timing: None,
            gate: Arc::new(Gate {
                mutex: Mutex::new(()),
                changed: Condvar::new(),
            }),
            #[cfg(test)]
            enrollments: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    #[cfg(feature = "witness-timing")]
    pub(crate) fn observed(mut self, timing: TimingObserver, kind: LockKind) -> Self {
        self.timing = Some((timing, kind));
        self
    }
    /// Additional wrapper state and Arc gate allocation layout, excluding allocator bookkeeping.
    #[cfg(feature = "witness-timing")]
    pub(crate) const fn additional_owned_bytes() -> usize {
        let header = 2 * std::mem::size_of::<usize>();
        let align = if std::mem::align_of::<Gate>() > std::mem::align_of::<usize>() {
            std::mem::align_of::<Gate>()
        } else {
            std::mem::align_of::<usize>()
        };
        let allocation = (header + std::mem::size_of::<Gate>()).next_multiple_of(align);
        std::mem::size_of::<Self>() - std::mem::size_of::<Mutex<T>>() + allocation
    }
    pub(crate) fn gate(&self) -> &Arc<Gate> {
        &self.gate
    }
    pub(crate) fn try_acquire(&self) -> Result<Option<Guard<'_, T>>, AcquireError> {
        let _gate = match self.gate.mutex.try_lock() {
            Ok(gate) => gate,
            Err(TryLockError::WouldBlock) => return Ok(None),
            Err(TryLockError::Poisoned(_)) => return Err(AcquireError::Poisoned),
        };
        self.try_data()
    }
    fn try_data(&self) -> Result<Option<Guard<'_, T>>, AcquireError> {
        match self.data.try_lock() {
            Ok(data) => Ok(Some(Guard {
                data: Some(data),
                gate: &self.gate,
            })),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Poisoned(_)) => Err(AcquireError::Poisoned),
        }
    }
    pub(crate) fn acquire(
        &self,
        until: Instant,
        stopped: &AtomicBool,
    ) -> Result<Guard<'_, T>, AcquireError> {
        let result = self.acquire_inner(until, stopped);
        #[cfg(feature = "witness-timing")]
        if let Some((timing, kind)) = &self.timing {
            timing.record(
                Metric::LockResult(
                    *kind,
                    match &result {
                        Ok(_) => LockOutcome::Acquired,
                        Err(AcquireError::Timeout) => LockOutcome::Timeout,
                        Err(AcquireError::Stopped) => LockOutcome::Stopped,
                        Err(AcquireError::Poisoned) => LockOutcome::Poisoned,
                    },
                ),
                std::time::Duration::ZERO,
            );
        }
        result
    }
    fn acquire_inner(
        &self,
        until: Instant,
        stopped: &AtomicBool,
    ) -> Result<Guard<'_, T>, AcquireError> {
        let mut gate = self.gate.mutex.lock().map_err(|_| AcquireError::Poisoned)?;
        loop {
            if stopped.load(Ordering::Acquire) {
                return Err(AcquireError::Stopped);
            }
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(AcquireError::Timeout);
            }
            if let Some(data) = self.try_data()? {
                drop(gate);
                if stopped.load(Ordering::Acquire) {
                    return Err(AcquireError::Stopped);
                }
                if Instant::now() >= until {
                    return Err(AcquireError::Timeout);
                }
                return Ok(data);
            }
            #[cfg(test)]
            self.enrollments.fetch_add(1, Ordering::Release);
            #[cfg(feature = "witness-timing")]
            if let Some((timing, kind)) = &self.timing {
                timing.record(Metric::LockRequested(*kind), left);
            }
            let outcome = {
                #[cfg(feature = "witness-timing")]
                let _span = self
                    .timing
                    .as_ref()
                    .map(|(timing, kind)| timing.span(Metric::LockActual(*kind)));
                self.gate.changed.wait_timeout(gate, left)
            };
            let (next, _) = outcome.map_err(|_| AcquireError::Poisoned)?;
            gate = next;
        }
    }
    #[cfg(test)]
    pub(crate) fn wait_for_enrollment(&self) {
        self.wait_for_enrollments(1);
    }
    #[cfg(test)]
    fn wait_for_enrollments(&self, n: usize) {
        let until = Instant::now() + std::time::Duration::from_secs(2);
        while self.enrollments.load(Ordering::Acquire) < n {
            assert!(Instant::now() < until, "waiter did not enroll");
            std::thread::yield_now();
        }
    }
}
