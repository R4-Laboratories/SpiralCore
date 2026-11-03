#![cfg(feature = "live")]
#[path = "../examples/service_sustained.rs"]
mod probe;

#[test]
fn concurrent_probe_keeps_independent_pairs_exact_and_accounts_actual_streams() {
    let row = probe::run_concurrent(8, 4, 0, false).unwrap();
    assert_eq!(row["workers"], 4);
    assert_eq!(row["pipeline_per_pair"], 1);
    assert_eq!(row["completed_operations"], 16);
    assert_eq!(row["exact_readbacks"], 24);
    assert_eq!(row["fidelity_failures"], 0);
    assert_eq!(row["sender_receiver_counters_match"], true);
    assert_eq!(row["incomplete_admitted_requests"], 0);
    assert!(row["actual_both_direction_bytes"].as_u64().unwrap() > 16 * 4096);
    assert_eq!(row["profiles"]["authorization_lifetime_ms"], 900_000);
}

#[test]
fn concurrent_probe_rejects_unbounded_work_before_allocating_endpoints() {
    assert!(probe::run_concurrent(3, 4, 0, false).is_err());
    assert!(probe::run_concurrent(128, 4, 0, false).is_err());
    assert!(probe::run_concurrent(2, 100_001, 0, false).is_err());
    assert!(probe::run_concurrent(2, 4, 361_000, false).is_err());
}

#[test]
fn interference_probe_keeps_healthy_pairs_moving_beside_invalid_and_slow_work() {
    let row = probe::run_concurrent(8, 4, 0, true).unwrap();
    assert_eq!(row["fidelity_failures"], 0);
    assert_eq!(row["workers"], 4);
    assert_eq!(row["completed_operations"], 16);
    assert_eq!(row["faults"]["malformed_refused"], 4);
    assert_eq!(row["faults"]["cancelled_refused"], 1);
    assert_eq!(row["faults"]["transport_receive_cancelled"], 1);
    assert!(row["per_worker"][1]["latency"]["p50_us"].as_u64().unwrap() >= 50_000);
    assert_eq!(row["sender_receiver_counters_match"], true);
}

#[test]
fn a_failed_worker_cannot_detach_later_workers() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let finished = Arc::new(AtomicUsize::new(0));
    let a = std::thread::spawn(|| Err::<usize, String>("expected worker failure".into()));
    let observed = finished.clone();
    let b = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(30));
        observed.fetch_add(1, Ordering::SeqCst);
        Ok(7)
    });
    assert!(probe::join_all(vec![a, b]).is_err());
    assert_eq!(finished.load(Ordering::SeqCst), 1);
}

#[test]
fn an_output_failure_aborts_the_probe_without_losing_worker_cleanup() {
    struct Fail;
    impl std::io::Write for Fail {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert!(probe::run_concurrent_recorded(8, 100, 1000, false, &mut Fail).is_err());
}
