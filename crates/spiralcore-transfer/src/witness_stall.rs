//! Optional observation of synchronous ordered Commit processing.
use std::{cell::RefCell, sync::Arc};

#[derive(Clone, Copy, Debug)]
pub enum CommitStage {
    Lookup,
    Quota,
    StageWrite,
    FileSync,
    NamespacePublish,
    VerifyPublished,
    OrderedReadback,
}

/// The worker that owns the sample word supplies this observer.
pub trait CommitObserver: Send + Sync {
    fn mark_stage(&self, stage: Option<CommitStage>);
}

struct Binding {
    observer: Arc<dyn CommitObserver>,
    stage: Option<CommitStage>,
}

thread_local! {
    static CURRENT: RefCell<Option<Binding>> = const { RefCell::new(None) };
}

struct RestoreBinding(Option<Binding>);
impl Drop for RestoreBinding {
    fn drop(&mut self) {
        // A nested binding may have reused the same receiver. Re-publish its
        // outer stage after the inner stage guards have been dropped.
        if let Some(binding) = &self.0 {
            binding.observer.mark_stage(binding.stage);
        }
        CURRENT.replace(self.0.take());
    }
}

pub fn with_observer<R>(observer: Arc<dyn CommitObserver>, operation: impl FnOnce() -> R) -> R {
    let _restore = RestoreBinding(CURRENT.replace(Some(Binding {
        observer,
        stage: None,
    })));
    operation()
}

struct RestoreStage {
    observer: Arc<dyn CommitObserver>,
    previous: Option<CommitStage>,
}
impl Drop for RestoreStage {
    fn drop(&mut self) {
        CURRENT.with_borrow_mut(|slot| {
            if let Some(binding) = slot {
                binding.stage = self.previous;
            }
        });
        self.observer.mark_stage(self.previous);
    }
}

pub fn with_stage<R>(stage: CommitStage, operation: impl FnOnce() -> R) -> R {
    let restore = CURRENT.with_borrow_mut(|slot| {
        slot.as_mut().map(|binding| RestoreStage {
            observer: Arc::clone(&binding.observer),
            previous: binding.stage.replace(stage),
        })
    });
    if let Some(guard) = &restore {
        guard.observer.mark_stage(Some(stage));
    }
    let result = operation();
    drop(restore);
    result
}
