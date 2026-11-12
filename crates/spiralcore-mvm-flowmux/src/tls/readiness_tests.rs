use super::*;
use std::{net::TcpListener, sync::mpsc};

fn raw_pair() -> (mio::net::TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let socket = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (peer, _) = listener.accept().unwrap();
    socket.set_nonblocking(true).unwrap();
    peer.set_write_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    (mio::net::TcpStream::from_std(socket), peer)
}

#[test]
fn readiness_preserves_preexisting_wake_data_eof_and_original_deadline() {
    let (mut socket, mut peer) = raw_pair();
    let readiness = Readiness::new(&mut socket).unwrap();
    readiness.notify().unwrap();
    let mut poller = readiness.poller().unwrap().unwrap();
    poller
        .wait(Instant::now() + Duration::from_secs(1))
        .unwrap();
    peer.write_all(b"exact").unwrap();
    poller
        .wait(Instant::now() + Duration::from_secs(1))
        .unwrap();
    let mut bytes = [0; 5];
    socket.read_exact(&mut bytes).unwrap();
    assert_eq!(&bytes, b"exact");
    let mut byte = [0];
    assert_eq!(
        socket.read(&mut byte).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    readiness.notify().unwrap();
    let until = Instant::now() + Duration::from_millis(30);
    loop {
        match poller.wait(until) {
            Ok(()) => assert!(Instant::now() < until),
            Err(e) => {
                assert_eq!(e.kind(), io::ErrorKind::TimedOut);
                break;
            }
        }
    }
    peer.shutdown(Shutdown::Write).unwrap();
    poller
        .wait(Instant::now() + Duration::from_secs(1))
        .unwrap();
    assert_eq!(socket.read(&mut byte).unwrap(), 0);
    assert_eq!(
        poller.wait(Instant::now()).unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
}

#[test]
fn readiness_one_owner_and_internal_notification_wake_parked_poll() {
    let (mut socket, _peer) = raw_pair();
    let readiness = Arc::new(Readiness::new(&mut socket).unwrap());
    let (entered, rx) = mpsc::sync_channel(1);
    std::thread::scope(|scope| {
        let other = readiness.clone();
        let waiter = scope.spawn(move || {
            let mut poller = other.poller().unwrap().unwrap();
            entered.send(()).unwrap();
            poller
                .wait(Instant::now() + Duration::from_secs(1))
                .unwrap();
        });
        rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(readiness.poller().unwrap().is_none());
        readiness.notify().unwrap();
        waiter.join().unwrap();
    });
}

thread_local! {
    static ENTER_POLL: std::cell::RefCell<Option<mpsc::Sender<()>>> = const { std::cell::RefCell::new(None) };
}
pub(super) fn before_poll() {
    ENTER_POLL.with(|slot| {
        if let Some(signal) = slot.borrow_mut().take() {
            let _ = signal.send(());
        }
    });
}
fn carriers() -> (TlsCarrier, TlsCarrier) {
    let identity = || {
        let key = ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new())
            .unwrap();
        PinnedIdentity::from_pkcs8(key.as_ref().to_vec()).unwrap()
    };
    let (a, b) = (identity(), identity());
    let (ap, bp) = (a.pin, b.pin);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let raw = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (peer, _) = listener.accept().unwrap();
    let until = Instant::now() + Duration::from_secs(2);
    std::thread::scope(|scope| {
        let server = scope.spawn(move || TlsCarrier::accept(peer, &b, ap, until).unwrap());
        let client = TlsCarrier::connect(raw, &a, bp, "peer.example", until).unwrap();
        let mut server = server.join().unwrap();
        let mut client = client;
        for carrier in [&mut client, &mut server] {
            carrier
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            carrier
                .set_write_timeout(Some(Duration::from_secs(1)))
                .unwrap();
        }
        (client, server)
    })
}

#[test]
fn peek_rechecks_buffered_tls_after_socket_and_internal_hints_are_consumed() {
    use std::io::BufRead;
    let (mut client, mut server) = carriers();
    server.write_all(b"fresh").unwrap();
    server.flush().unwrap();
    let until = Instant::now() + Duration::from_secs(1);
    loop {
        let ready = client
            .with_session(|session| {
                pump(session, &client.shared.received, &client.shared.sent)?;
                match session.tls.reader().fill_buf() {
                    Ok(bytes) => Ok(!bytes.is_empty()),
                    Err(e) if transient(&e) => Ok(false),
                    Err(e) => Err(e),
                }
            })
            .unwrap();
        if ready {
            break;
        }
        pause(until).unwrap();
    }
    // Another clone has already consumed the encrypted socket readiness and
    // left plaintext in TLS. Remove old readiness/waker hints before peek.
    client
        .with_session(|session| {
            let mut byte = [0];
            assert_eq!(
                session.socket.peek(&mut byte).unwrap_err().kind(),
                io::ErrorKind::WouldBlock
            );
            Ok(())
        })
        .unwrap();
    {
        let mut poller = client.shared.readiness.poller().unwrap().unwrap();
        for _ in 0..16 {
            let empty = {
                let PollState { poll, events, .. } = &mut *poller;
                events.clear();
                poll.poll(events, Some(Duration::ZERO)).unwrap();
                events.is_empty()
            };
            poller.observe_wake();
            if empty {
                break;
            }
        }
    }
    let mut bytes = [0; 5];
    assert_eq!(client.peek(&mut bytes).unwrap(), 5);
    assert_eq!(&bytes, b"fresh");
    client.read_exact(&mut bytes).unwrap();
    assert_eq!(&bytes, b"fresh");
    server.shutdown(Shutdown::Write).unwrap();
    assert_eq!(client.peek(&mut bytes).unwrap(), 0);
}

#[test]
fn competing_peek_readers_preserve_exact_bytes_and_shutdown_wakes_wait_entry() {
    let (mut client, mut server) = carriers();
    let (ready, rx) = mpsc::channel();
    std::thread::scope(|scope| {
        let mut a = client.try_clone().unwrap();
        let mut b = client.try_clone().unwrap();
        let ar = ready.clone();
        let read = |carrier: &mut TlsCarrier, signal: mpsc::Sender<()>| {
            signal.send(()).unwrap();
            let mut byte = [0];
            assert_eq!(carrier.peek(&mut byte).unwrap(), 1);
            carrier.read_exact(&mut byte).unwrap();
            byte[0]
        };
        let a = scope.spawn(move || read(&mut a, ar));
        let b = scope.spawn(move || read(&mut b, ready));
        rx.recv_timeout(Duration::from_secs(1)).unwrap();
        rx.recv_timeout(Duration::from_secs(1)).unwrap();
        server.write_all(b"ab").unwrap();
        server.flush().unwrap();
        let mut actual = [a.join().unwrap(), b.join().unwrap()];
        actual.sort_unstable();
        assert_eq!(&actual, b"ab");
    });
    let control = client.try_clone().unwrap();
    let (entered, rx) = mpsc::channel();
    std::thread::scope(|scope| {
        let waiter = scope.spawn(move || {
            ENTER_POLL.with(|slot| *slot.borrow_mut() = Some(entered));
            client.peek(&mut [0]).unwrap_err().kind()
        });
        // Synchronized at poll entry: shutdown can race just before or during
        // the kernel wait. It must wake either ordering, without a sleep.
        rx.recv_timeout(Duration::from_secs(1)).unwrap();
        control.shutdown(Shutdown::Both).unwrap();
        assert_eq!(waiter.join().unwrap(), io::ErrorKind::BrokenPipe);
    });
}

thread_local! {
    static WAKE_POSTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static FAIL_WAKE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
pub(super) fn before_wake() -> io::Result<()> {
    WAKE_POSTS.with(|count| count.set(count.get() + 1));
    if FAIL_WAKE.with(std::cell::Cell::get) {
        Err(io::Error::other("injected readiness wake failure"))
    } else {
        Ok(())
    }
}

#[test]
fn notification_storm_posts_once_until_observed_wake_then_can_post_again() {
    let (mut socket, _peer) = raw_pair();
    let readiness = Readiness::new(&mut socket).unwrap();
    WAKE_POSTS.with(|count| count.set(0));
    for _ in 0..1000 {
        readiness.notify().unwrap();
    }
    assert_eq!(
        WAKE_POSTS.with(std::cell::Cell::get),
        1,
        "actual Waker posts must be bounded even without a poll consumer"
    );
    let mut poller = readiness.poller().unwrap().unwrap();
    poller
        .wait(Instant::now() + Duration::from_secs(1))
        .unwrap();
    for _ in 0..1000 {
        readiness.notify().unwrap();
    }
    assert_eq!(WAKE_POSTS.with(std::cell::Cell::get), 2);
}

#[test]
fn failed_wake_post_propagates_and_fences_waiting_carrier_without_stuck_flag() {
    struct Failure;
    impl Drop for Failure {
        fn drop(&mut self) {
            FAIL_WAKE.with(|flag| flag.set(false));
        }
    }
    let (mut client, _server) = carriers();
    let mut control = client.try_clone().unwrap();
    // Consume the handshake hint so the next notify must actually post.
    control.shared.readiness.notify().unwrap();
    control
        .shared
        .readiness
        .poller()
        .unwrap()
        .unwrap()
        .wait(Instant::now() + Duration::from_secs(1))
        .unwrap();
    let (entered, rx) = mpsc::channel();
    std::thread::scope(|scope| {
        let waiter = scope.spawn(move || {
            ENTER_POLL.with(|slot| *slot.borrow_mut() = Some(entered));
            client.peek(&mut [0]).unwrap_err().kind()
        });
        rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(!control.shared.readiness.notified.load(Ordering::Acquire));
        FAIL_WAKE.with(|flag| flag.set(true));
        let _failure = Failure;
        assert_eq!(control.flush().unwrap_err().kind(), io::ErrorKind::Other);
        assert!(control.shared.stopped.load(Ordering::Acquire));
        assert!(!control.shared.readiness.notified.load(Ordering::Acquire));
        assert_eq!(waiter.join().unwrap(), io::ErrorKind::BrokenPipe);
        assert!(control.try_clone().is_err());
    });
}

#[test]
fn stopped_waiter_observes_fence_without_socket_or_waker_event() {
    let (mut socket, _peer) = raw_pair();
    let readiness = Arc::new(Readiness::new(&mut socket).unwrap());
    let stopped = AtomicBool::new(false);
    let (entered, entry) = mpsc::channel();
    let (result, finished) = mpsc::channel();
    std::thread::scope(|scope| {
        let readiness = &readiness;
        let stopped = &stopped;
        scope.spawn(move || {
            ENTER_POLL.with(|slot| *slot.borrow_mut() = Some(entered));
            let mut poller = readiness.poller().unwrap().unwrap();
            let until = Instant::now() + Duration::from_secs(2);
            let outcome = poller.wait_checked(until, || {
                if stopped.load(Ordering::Acquire) {
                    Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "closed by original owner",
                    ))
                } else {
                    Ok(())
                }
            });
            result.send(outcome.unwrap_err().kind()).unwrap();
        });
        entry.recv_timeout(Duration::from_secs(1)).unwrap();
        stopped.store(true, Ordering::Release);
        assert_eq!(
            finished.recv_timeout(Duration::from_millis(500)).unwrap(),
            io::ErrorKind::BrokenPipe,
        );
    });
}

#[test]
fn peek_releases_poller_before_waiting_for_session_and_owner_stop_wakes_it() {
    let (client, _server) = carriers();
    let mut reader = client.try_clone().unwrap();
    let held = client.shared.session.try_acquire().unwrap().unwrap();
    std::thread::scope(|scope| {
        let waiter = scope.spawn(move || reader.peek(&mut [0]));
        client.shared.session.wait_for_enrollment();
        // An enrolled Session waiter cannot retain the sole readiness poller.
        assert!(client.shared.readiness.poller().unwrap().is_some());
        client.shared.stop();
        assert_eq!(
            waiter.join().unwrap().unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
        drop(held);
    });
}
