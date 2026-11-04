#![cfg(all(unix, feature = "legacy-interop"))]
//! Uses a separately built, unmodified frozen upstream peer. The helper is
//! outside this crate's path-patched dependency closure and never boots a VM.
use ed25519_dalek::SigningKey;
use mvm_agentd::vsock::{
    AuthenticatedSession, GuestRequest, GuestResponse, protocol_hello_response,
};
use spiralcore_mvm_access::{Error, OwnerClient};
use std::{
    io::{BufRead, BufReader},
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread::JoinHandle,
    time::{Duration, Instant},
};

struct Peer {
    child: Child,
    reader: Option<JoinHandle<()>>,
    lines: mpsc::Receiver<String>,
}
impl Peer {
    fn start(arguments: &[&str]) -> Self {
        let executable = std::env::var_os("SPIRALCORE_ACCESS_LEGACY_PEER")
            .expect("explicit frozen helper binary required");
        let mut child = Command::new(executable)
            .args(arguments)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let output = child.stdout.take().unwrap();
        let (send, lines) = mpsc::sync_channel(4);
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(output).lines().take(4) {
                let Ok(line) = line else { break };
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            reader: Some(reader),
            lines,
        }
    }
    fn line(&self) -> String {
        self.lines
            .recv_timeout(Duration::from_secs(5))
            .expect("bounded frozen peer event")
    }
    fn finish(&mut self) {
        assert!(self.child.wait().unwrap().success());
        if let Some(reader) = self.reader.take() {
            reader.join().unwrap();
        }
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

struct AcceptObserver {
    address: std::net::SocketAddr,
    accepted: mpsc::Receiver<std::io::Result<TcpStream>>,
    worker: Option<JoinHandle<()>>,
}
impl AcceptObserver {
    fn arm() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (send, accepted) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            let _ = send.send(listener.accept().map(|(stream, _)| stream));
        });
        Self {
            address,
            accepted,
            worker: Some(worker),
        }
    }
    fn wait(&mut self) -> TcpStream {
        let stream = self
            .accepted
            .recv_timeout(Duration::from_secs(5))
            .expect("bounded owned accept")
            .unwrap();
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
        stream
    }
}
impl Drop for AcceptObserver {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            if !worker.is_finished() {
                let _ = TcpStream::connect_timeout(&self.address, Duration::from_millis(200));
            }
            let _ = worker.join();
        }
    }
}

#[test]
fn actual_frozen_old_host_reads_new_guest_ack_and_legacy_ping() {
    let mut accept = AcceptObserver::arm();
    let address = accept.address.to_string();
    let mut peer = Peer::start(&["client", &address]);
    let mut stream = accept.wait();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let host = SigningKey::from_bytes(&[31; 32]);
    let mut session = AuthenticatedSession::guest(
        &mut stream,
        SigningKey::from_bytes(&[32; 32]),
        &host.verifying_key(),
    )
    .unwrap();
    let req: GuestRequest = session.read(&mut stream).unwrap();
    let GuestRequest::ProtocolHello {
        host_protocol_version,
        min_supported_version,
        host_version,
        requested_capabilities,
    } = req
    else {
        panic!("expected legacy hello")
    };
    session
        .write(
            &mut stream,
            &protocol_hello_response(
                host_protocol_version,
                min_supported_version,
                &host_version,
                &requested_capabilities,
            ),
        )
        .unwrap();
    assert!(matches!(
        session.read::<GuestRequest>(&mut stream).unwrap(),
        GuestRequest::Ping
    ));
    session.write(&mut stream, &GuestResponse::Pong).unwrap();
    assert_eq!(peer.line(), "LEGACY_CLIENT_OK");
    peer.finish();
}

#[test]
fn new_owner_client_refuses_actual_frozen_old_guest_before_any_operation() {
    let mut peer = Peer::start(&["server"]);
    let ready = peer.line();
    let address = ready
        .strip_prefix("READY ")
        .expect("explicit frozen listener event")
        .parse()
        .unwrap();
    let socket = TcpStream::connect_timeout(&address, Duration::from_secs(2)).unwrap();
    let result = OwnerClient::connect(
        socket,
        "frozen-guest-component",
        SigningKey::from_bytes(&[31; 32]),
        &SigningKey::from_bytes(&[32; 32]).verifying_key(),
        Instant::now() + Duration::from_secs(3),
    );
    assert!(matches!(result, Err(Error::Unsupported)));
    assert_eq!(peer.line(), "LEGACY_SERVER_OK");
    peer.finish();
}
