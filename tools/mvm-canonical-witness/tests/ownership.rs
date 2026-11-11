use spiralcore_mvm_canonical_witness::{cleanup_after_launch, preflight_failure};
use std::{
    cell::Cell,
    future::Future,
    task::{Context, Poll, Waker},
};
fn finish<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("controlled cleanup future unexpectedly blocked"),
    }
}
#[test]
fn failed_launch_without_original_outcome_never_stops_or_cleans_by_name() {
    let calls = Cell::new(0);
    let result = finish(cleanup_after_launch::<()>(
        None,
        async {
            calls.set(calls.get() + 1);
            Ok(())
        },
        async {
            calls.set(calls.get() + 10);
            Ok(())
        },
    ));
    assert_eq!(calls.get(), 0);
    assert_eq!(result.stop, "NOT_AUTHORIZED");
    assert_eq!(result.cleanup, "RETAINED_UNOWNED_LAUNCH");
}
#[test]
fn original_successful_launch_still_stops_after_receive_or_capture_failure() {
    let calls = Cell::new(0);
    let result = finish(cleanup_after_launch(
        Some(&()),
        async {
            calls.set(1);
            Ok(())
        },
        async {
            calls.set(calls.get() + 10);
            Ok(())
        },
    ));
    assert_eq!(calls.get(), 11);
    assert_eq!(result.cleanup, "canonical_cleanup_ok");
}
#[test]
fn stop_refusal_retains_cleanup_without_polling_another_owner() {
    let calls = Cell::new(0);
    let result = finish(cleanup_after_launch(
        Some(&()),
        async { Err(anyhow::anyhow!("original stop refused")) },
        async {
            calls.set(1);
            Ok(())
        },
    ));
    assert_eq!(calls.get(), 0);
    assert_eq!(result.cleanup, "RETAINED_AFTER_STOP_REFUSAL");
    assert!(result.error.unwrap().contains("original stop refused"));
}
#[test]
fn preflight_failure_is_explicit_json_without_invented_transfer_counts() {
    let result = preflight_failure("missing rootfs");
    assert_eq!(result["status"], "failed_preflight");
    assert_eq!(result["native_transfer"], "NOT_RUN");
    assert!(result["received_bytes"].is_null());
    assert_eq!(result["error"], "missing rootfs");
}
