//! Optional observation of synchronous ordered Commit processing.
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

pub fn with_stage<R>(_stage: CommitStage, operation: impl FnOnce() -> R) -> R {
    operation()
}
