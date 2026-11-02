//! A dropped authenticated stream must not turn a partial object into a publication.
use spiralcore_mvm_adapter::live::{HandshakeRole, SocketIo};
use spiralcore_mvm_live::{
    campaign,
    config::{Arm, Pair, Plan, Published, Stage},
};
use std::{
    collections::BTreeSet,
    io::{self, Read, Write},
    net::Shutdown,
    os::unix::net::UnixStream,
    time::Duration,
};

struct CutWrite {
    socket: UnixStream,
    remaining: usize,
}
impl Read for CutWrite {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.socket.read(buffer)
    }
}
impl Write for CutWrite {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            self.socket.shutdown(Shutdown::Both)?;
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let count = self
            .socket
            .write(&bytes[..bytes.len().min(self.remaining)])?;
        self.remaining -= count;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.socket.flush()
    }
}
impl SocketIo for CutWrite {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        SocketIo::set_read_timeout(&self.socket, timeout)
    }
    fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        SocketIo::set_write_timeout(&self.socket, timeout)
    }
    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.socket.shutdown(how)
    }
}
fn rows(bytes: &[u8]) -> Vec<serde_json::Value> {
    bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect()
}

#[test]
fn interrupted_combined_object_is_unpublished_and_fresh_stream_recovers_exactly() {
    let arm: Arm = serde_json::from_str("\"combined-cpu\"").unwrap();
    let pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm,
        size: 1 << 20,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    let store = tempfile::tempdir().unwrap();
    let cache = store.path().join("cache");
    let (send, receive) = UnixStream::pair().unwrap();
    let (sender_rows, receiver_rows) = std::thread::scope(|scope| {
        let receiver = scope.spawn(|| {
            let mut log = Vec::new();
            assert!(
                campaign::receiver(
                    &pair.receiver,
                    &cache,
                    receive,
                    HandshakeRole::Acceptor,
                    &mut log,
                )
                .is_err()
            );
            log
        });
        let mut log = Vec::new();
        assert!(
            campaign::sender(
                &pair.sender,
                CutWrite {
                    socket: send,
                    remaining: 256 << 10,
                },
                HandshakeRole::Initiator,
                &mut log,
            )
            .is_err()
        );
        (rows(&log), rows(&receiver.join().unwrap()))
    });
    let committed: Vec<_> = receiver_rows
        .iter()
        .filter(|row| row["event"] == "receiver_operation")
        .collect();
    assert_eq!(
        committed
            .iter()
            .map(|row| row["kind"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "runtime-checkpoint",
            "schema",
            "generated",
            "residual",
            "repeat"
        ]
    );
    assert!(!sender_rows.iter().any(|row| row["event"] == "sender_done"));
    assert!(
        !receiver_rows
            .iter()
            .any(|row| row["event"] == "receiver_done")
    );
    let expected: BTreeSet<_> = committed
        .iter()
        .map(|row| format!("{}.object", row["object"].as_str().unwrap()))
        .collect();
    let mut present = BTreeSet::new();
    for entry in std::fs::read_dir(&cache).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            for file in std::fs::read_dir(entry.path()).unwrap() {
                let name = file.unwrap().file_name().into_string().unwrap();
                if name.ends_with(".object") {
                    present.insert(name);
                }
            }
        }
    }
    assert_eq!(
        present, expected,
        "interrupted object must not be published"
    );

    // New actors and transport discard all readiness assumptions. The same
    // still-valid capability/epoch uses fresh envelope IDs, not a durable inbox.
    let (send, receive) = UnixStream::pair().unwrap();
    let received = std::thread::scope(|scope| {
        let receiver = scope.spawn(|| {
            campaign::receiver(
                &pair.receiver,
                &cache,
                receive,
                HandshakeRole::Acceptor,
                &mut Vec::new(),
            )
            .unwrap()
        });
        let sent = campaign::sender(
            &pair.sender,
            send,
            HandshakeRole::Initiator,
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(sent["operations"], 10);
        receiver.join().unwrap()
    });
    assert_eq!(received["exact_reopened"], true);
    let published: Vec<Published> = serde_json::from_value(received["published"].clone()).unwrap();
    assert_eq!(published.len(), 10);
    campaign::verify(&pair.sender.plan, &cache, &published, &mut Vec::new()).unwrap();
}
