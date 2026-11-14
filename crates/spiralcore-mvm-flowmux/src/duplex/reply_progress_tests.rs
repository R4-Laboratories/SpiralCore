//! Trusted connected-transport control; no authenticated MVM or WAN is fabricated.
use super::*;
use spiralcore_transfer::{
    resource_profile::ResourceProfile,
    v1::{Capabilities, ObjectMode},
};
use std::net::{TcpListener, TcpStream};

struct Gate {
    armed: AtomicBool,
    entered: SyncSender<()>,
    release: Mutex<Receiver<()>>,
}
struct GateAuthority {
    gate: Arc<Gate>,
    hold: bool,
}
impl Authority for GateAuthority {
    fn authorize(&self, _: &Context) -> spiralcore_transfer::Result<()> {
        if self.hold && self.gate.armed.swap(false, Ordering::AcqRel) {
            self.gate
                .entered
                .try_send(())
                .map_err(|_| spiralcore_transfer::Error::Storage)?;
            self.gate
                .release
                .lock()
                .map_err(|_| spiralcore_transfer::Error::Storage)?
                .recv_timeout(Duration::from_secs(3))
                .map_err(|_| spiralcore_transfer::Error::Storage)?;
        }
        Ok(())
    }
}
struct Release(Option<SyncSender<()>>);
impl Release {
    fn fire(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.try_send(());
        }
    }
}
impl Drop for Release {
    fn drop(&mut self) {
        self.fire();
    }
}
struct Store(std::path::PathBuf);
impl Drop for Store {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn independent_reply_progresses_while_original_inbound_authority_is_held() {
    reply_progress(false);
}
#[test]
fn held_inbound_turn_paces_the_next_send_without_publishing_it() {
    reply_progress(true);
}
fn reply_progress(pace: bool) {
    let mut context = super::tests::context();
    context.capability = "independent-reply-progress".into();
    let binding = Binding {
        flow: context.namespace().unwrap(),
        policy_sha256: [4; 32],
        scope_commitment: [9; 32],
    };
    let caps = Capabilities::small_v1();
    let profile = caps.negotiate(&caps).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).unwrap();
    let (server, peer) = listener.accept().unwrap();
    let stores = ["left", "right"].map(|side| {
        Store(std::env::temp_dir().join(format!(
            "reply-progress-{pace}-{side}-{}",
            std::process::id()
        )))
    });
    let (entered, entered_rx) = mpsc::sync_channel(1);
    let (release, released) = mpsc::sync_channel(1);
    let gate = Arc::new(Gate {
        armed: AtomicBool::new(false),
        entered,
        release: Mutex::new(released),
    });
    let make = |socket, peer, role, root: &Store, timeout| {
        let channel =
            ObjectChannel::from_connected(socket, binding, peer, timeout, caps, caps).unwrap();
        let sender = ObjectSender::new_with_resource_profile(
            ObjectMode::Whole,
            &context,
            None,
            profile,
            ResourceProfile::SmallV1,
        )
        .unwrap();
        let endpoint = ObjectEndpoint::open_with_resource_profile(
            &root.0,
            ResourceProfile::SmallV1.receiver_limits(),
            GateAuthority {
                gate: gate.clone(),
                hold: role == Role::Initiator,
            },
            profile,
            ResourceProfile::SmallV1,
        )
        .unwrap();
        Duplex::spawn(channel, sender, endpoint, context.clone(), role)
            .unwrap()
            .split()
    };
    let (mut left_tx, mut left_rx, mut left_guard) = make(
        client,
        address,
        Role::Initiator,
        &stores[0],
        Duration::from_millis(500),
    );
    let (mut right_tx, mut right_rx, mut right_guard) = make(
        server,
        peer,
        Role::Acceptor,
        &stores[1],
        Duration::from_secs(2),
    );
    let left_cancel = left_tx.cancellation.clone();
    let right_cancel = right_tx.cancellation.clone();
    let wire = right_tx.tx.clone();
    let local_wire = left_tx.tx.clone();
    let diagnostic_stop = AtomicBool::new(false);
    let snapshot = |wire: &SharedTx| {
        wire.acquire(Instant::now() + Duration::from_secs(2), &diagnostic_stop)
            .ok()
            .map(|tx| tx.counters())
    };
    gate.armed.store(true, Ordering::Release);
    let mut release = Release(Some(release));
    std::thread::scope(|scope| {
        let peer_sender = scope.spawn(move || {
            let result = right_tx.send_block(vec![0xb2; 1024]);
            let half_close = right_tx.half_close();
            (result, half_close)
        });
        let entered = entered_rx.recv_timeout(Duration::from_secs(2));
        let before = snapshot(&wire);
        let local_result = if entered.is_ok() {
            left_tx.send_block(vec![0xa1; 1024])
        } else {
            Err(Error::Worker)
        };
        let after = snapshot(&wire);
        let (local_half_close, extra, paced) = if pace && local_result.is_ok() {
            let routing = left_tx.routing.clone();
            let enrolled = routing.enrollments();
            let before_extra = snapshot(&local_wire);
            let sender = scope.spawn(move || {
                let extra = left_tx.send_block(vec![0xc3; 1024]);
                let closed = left_tx.half_close();
                (extra, closed)
            });
            let waited = routing
                .wait_for_enrollment_after(enrolled, Instant::now() + Duration::from_millis(200));
            let after_extra = snapshot(&local_wire);
            release.fire();
            let (extra, closed) = sender.join().unwrap();
            (
                closed,
                Some(extra),
                Some((waited, before_extra, after_extra)),
            )
        } else {
            release.fire();
            let closed = left_tx.half_close();
            drop(left_tx);
            (closed, None, None)
        };
        let peer_result = peer_sender.join();
        let left_data = left_rx.receive();
        let right_data = right_rx.receive();
        let right_extra = if pace { Some(right_rx.receive()) } else { None };
        let left_fin = left_rx.receive();
        let right_fin = right_rx.receive();
        drop(left_rx);
        drop(right_rx);
        let left_collection = left_guard.try_join_until(Instant::now() + Duration::from_secs(2));
        let right_collection = right_guard.try_join_until(Instant::now() + Duration::from_secs(2));
        let collected = left_guard.child.is_none()
            && left_guard.joined.is_some()
            && right_guard.child.is_none()
            && right_guard.joined.is_some();
        let left_semantics = left_guard.join();
        let right_semantics = right_guard.join();
        eprintln!(
            "FAILURES left={:?} right={:?}",
            left_cancel.first_failure(),
            right_cancel.first_failure()
        );
        eprintln!(
            "REPLY_PROGRESS inbound_gate_entered={} before={before:?} after={after:?} local_success={} original_workers_collected={collected} left_collection={left_collection:?} right_collection={right_collection:?} peer_result={peer_result:?} left_semantics={left_semantics:?} right_semantics={right_semantics:?}",
            entered.is_ok(),
            local_result.is_ok()
        );
        // All owned callers/workers are collected and the gate is released before assertions.
        assert!(entered.is_ok(), "original inbound gate must be reached");
        assert!(
            collected,
            "original worker handles must be physically collected"
        );
        let before = before.expect("original peer TX counter before local call");
        let after = after.expect("original peer TX counter after local call");
        assert!(
            after.records > before.records,
            "peer must complete its reply while inbound remains held"
        );
        assert!(
            local_result.is_ok(),
            "independent reply must progress while inbound is held: {local_result:?}"
        );
        if pace {
            let (waited, before_extra, after_extra) = paced.expect("second operation control");
            assert!(
                waited,
                "second caller must enroll while inbound remains held"
            );
            assert_eq!(
                before_extra, after_extra,
                "paced operation must publish no request bytes"
            );
            extra.unwrap().unwrap();
            assert_eq!(right_extra.unwrap().unwrap(), Event::Data(vec![0xc3; 1024]));
        }
        local_half_close.unwrap();
        let (sent, closed) = peer_result.unwrap();
        sent.unwrap();
        closed.unwrap();
        assert_eq!(left_data.unwrap(), Event::Data(vec![0xb2; 1024]));
        assert_eq!(right_data.unwrap(), Event::Data(vec![0xa1; 1024]));
        assert_eq!(left_fin.unwrap(), Event::HalfClose);
        assert_eq!(right_fin.unwrap(), Event::HalfClose);
        left_semantics.unwrap();
        right_semantics.unwrap();
    });
}
