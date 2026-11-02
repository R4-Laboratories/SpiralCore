//! One reliable object stream on the separate pinned-mTLS object profile.
use crate::transport::{self, Peer, UdpCounters};
use spiralcore_transfer::{protocol::Frame, wan};
use std::future::Future;
use tokio::time::{timeout_at, Instant};
use tokio_util::sync::CancellationToken;

pub const MAX_BATCH_BYTES: usize = wan::MAX_BATCH;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("object transport: {0}")]
    Transport(#[from] transport::Error),
    #[error("object batch: {0}")]
    Batch(#[from] spiralcore_transfer::Error),
    #[error("object stream read: {0}")]
    Read(#[from] quinn::ReadExactError),
    #[error("object stream end: {0}")]
    End(#[from] quinn::ReadError),
    #[error("object stream write: {0}")]
    Write(#[from] quinn::WriteError),
    #[error("object stream closed: {0}")]
    StreamClosed(#[from] quinn::ClosedStream),
    #[error("object stream acknowledgement: {0}")]
    Stopped(#[from] quinn::StoppedError),
    #[error("object stream was stopped by the peer")]
    PeerStopped,
    #[error("object channel deadline expired")]
    Deadline,
    #[error("object channel operation cancelled")]
    Cancelled,
    #[error("object batch length exceeds bounds")]
    Length,
    #[error("unexpected object channel end or data after end")]
    UnexpectedEnd,
    #[error("object channel is closed or an earlier operation was interrupted")]
    Poisoned,
}
pub type Result<T> = std::result::Result<T, Error>;

/// Completed logical prefix/body operations, not UDP or retransmission bytes.
/// Failed partial I/O remains visible in Quinn counters and independent capture.
#[derive(Debug, Default, Clone, Copy, serde::Serialize)]
pub struct ChannelCounters {
    pub encoded_batch_bytes_sent: u64,
    pub encoded_batch_bytes_received: u64,
    pub framing_bytes_sent: u64,
    pub framing_bytes_received: u64,
    pub batches_sent: u64,
    pub batches_received: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Open,
    Busy,
    EndReceived,
    Finished,
    Poisoned,
}

pub struct ObjectChannel {
    peer: Peer,
    send: quinn::SendStream,
    receive: quinn::RecvStream,
    state: State,
    counters: ChannelCounters,
}

fn check(deadline: Instant, cancel: &CancellationToken) -> Result<()> {
    if cancel.is_cancelled() {
        Err(Error::Cancelled)
    } else if Instant::now() >= deadline {
        Err(Error::Deadline)
    } else {
        Ok(())
    }
}

async fn bounded<T>(
    deadline: Instant,
    cancel: &CancellationToken,
    operation: impl Future<Output = Result<T>>,
) -> Result<T> {
    check(deadline, cancel)?;
    let value = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(Error::Cancelled),
        result = timeout_at(deadline, operation) => result.map_err(|_| Error::Deadline)?,
    }?;
    check(deadline, cancel)?;
    Ok(value)
}

impl Peer {
    /// Consumes the peer; the application cannot create a second stream.
    pub async fn open_object_channel(
        self,
        deadline: Instant,
        cancel: &CancellationToken,
    ) -> Result<ObjectChannel> {
        let streams = bounded(deadline, cancel, async {
            self.object_connection()?
                .open_bi()
                .await
                .map_err(transport::Error::Connection)
                .map_err(Error::Transport)
        })
        .await;
        ObjectChannel::from_streams(self, streams)
    }
    pub async fn accept_object_channel(
        self,
        deadline: Instant,
        cancel: &CancellationToken,
    ) -> Result<ObjectChannel> {
        let streams = bounded(deadline, cancel, async {
            self.object_connection()?
                .accept_bi()
                .await
                .map_err(transport::Error::Connection)
                .map_err(Error::Transport)
        })
        .await;
        ObjectChannel::from_streams(self, streams)
    }
}

impl ObjectChannel {
    fn from_streams(
        peer: Peer,
        streams: Result<(quinn::SendStream, quinn::RecvStream)>,
    ) -> Result<Self> {
        match streams {
            Ok((send, receive)) => Ok(Self {
                peer,
                send,
                receive,
                state: State::Open,
                counters: ChannelCounters::default(),
            }),
            Err(error) => {
                peer.abort();
                Err(error)
            }
        }
    }
    fn poison(&mut self) {
        self.state = State::Poisoned;
        self.peer.abort();
    }
    fn begin(&mut self, expected: State) -> Result<()> {
        if self.state != expected {
            self.poison();
            return Err(Error::Poisoned);
        }
        // A dropped future leaves Busy; later use must never resume partial I/O.
        self.state = State::Busy;
        Ok(())
    }
    fn conclude<T>(&mut self, result: Result<T>, next: State) -> Result<T> {
        if result.is_ok() {
            self.state = next;
        } else {
            self.poison();
        }
        result
    }
    async fn write_prefix(
        &mut self,
        length: u32,
        deadline: Instant,
        cancel: &CancellationToken,
    ) -> Result<()> {
        bounded(deadline, cancel, async {
            self.send.write_all(&length.to_le_bytes()).await?;
            Ok(())
        })
        .await?;
        self.counters.framing_bytes_sent += 4;
        Ok(())
    }
    async fn write_batch(
        &mut self,
        frames: &[Frame],
        deadline: Instant,
        cancel: &CancellationToken,
    ) -> Result<()> {
        check(deadline, cancel)?;
        let bytes = wan::encode_batch(frames)?;
        self.write_prefix(bytes.len() as u32, deadline, cancel)
            .await?;
        bounded(deadline, cancel, async {
            self.send.write_all(&bytes).await?;
            Ok(())
        })
        .await?;
        self.counters.encoded_batch_bytes_sent += bytes.len() as u64;
        self.counters.batches_sent += 1;
        Ok(())
    }
    async fn read_batch(
        &mut self,
        deadline: Instant,
        cancel: &CancellationToken,
    ) -> Result<Option<Vec<Frame>>> {
        let mut prefix = [0; 4];
        bounded(deadline, cancel, async {
            self.receive.read_exact(&mut prefix).await?;
            Ok(())
        })
        .await?;
        self.counters.framing_bytes_received += 4;
        let length = u32::from_le_bytes(prefix) as usize;
        if length == 0 {
            return Ok(None);
        }
        if !(12..=MAX_BATCH_BYTES).contains(&length) {
            return Err(Error::Length);
        }
        let mut bytes = vec![0; length];
        bounded(deadline, cancel, async {
            self.receive.read_exact(&mut bytes).await?;
            Ok(())
        })
        .await?;
        self.counters.encoded_batch_bytes_received += length as u64;
        let frames = wan::decode_batch(&bytes)?;
        check(deadline, cancel)?;
        self.counters.batches_received += 1;
        Ok(Some(frames))
    }
    pub async fn send_batch(
        &mut self,
        frames: &[Frame],
        deadline: Instant,
        cancel: &CancellationToken,
    ) -> Result<()> {
        self.begin(State::Open)?;
        let result = self.write_batch(frames, deadline, cancel).await;
        self.conclude(result, State::Open)
    }
    pub async fn recv_batch(
        &mut self,
        deadline: Instant,
        cancel: &CancellationToken,
    ) -> Result<Option<Vec<Frame>>> {
        self.begin(State::Open)?;
        let result = self.read_batch(deadline, cancel).await;
        let next = if matches!(result, Ok(None)) {
            State::EndReceived
        } else {
            State::Open
        };
        self.conclude(result, next)
    }
    /// Application correlation and terminal-prefix replies stay with the object client.
    pub async fn exchange(
        &mut self,
        frames: &[Frame],
        deadline: Instant,
        cancel: &CancellationToken,
    ) -> Result<Vec<Frame>> {
        self.begin(State::Open)?;
        let result = async {
            self.write_batch(frames, deadline, cancel).await?;
            self.read_batch(deadline, cancel)
                .await?
                .ok_or(Error::UnexpectedEnd)
        }
        .await;
        self.conclude(result, State::Open)
    }
    async fn receive_fin(&mut self, deadline: Instant, cancel: &CancellationToken) -> Result<()> {
        bounded(deadline, cancel, async {
            let mut extra = [0; 1];
            if self.receive.read(&mut extra).await?.is_some() {
                return Err(Error::UnexpectedEnd);
            }
            Ok(())
        })
        .await
    }
    async fn acknowledged(&mut self, deadline: Instant, cancel: &CancellationToken) -> Result<()> {
        bounded(deadline, cancel, async {
            if self.send.stopped().await?.is_some() {
                return Err(Error::PeerStopped);
            }
            Ok(())
        })
        .await
    }
    /// Sends orderly end, finishes writing, and requires the peer's explicit end ACK.
    pub async fn finish(&mut self, deadline: Instant, cancel: &CancellationToken) -> Result<()> {
        self.begin(State::Open)?;
        let result = async {
            self.write_prefix(0, deadline, cancel).await?;
            self.send.finish()?;
            if self.read_batch(deadline, cancel).await?.is_some() {
                return Err(Error::UnexpectedEnd);
            }
            self.receive_fin(deadline, cancel).await?;
            self.acknowledged(deadline, cancel).await
        }
        .await;
        self.conclude(result, State::Finished)
    }
    /// Only valid after recv_batch returned None. Waits for transport delivery of the ACK.
    pub async fn acknowledge_finish(
        &mut self,
        deadline: Instant,
        cancel: &CancellationToken,
    ) -> Result<()> {
        self.begin(State::EndReceived)?;
        let result = async {
            self.receive_fin(deadline, cancel).await?;
            self.write_prefix(0, deadline, cancel).await?;
            self.send.finish()?;
            self.acknowledged(deadline, cancel).await
        }
        .await;
        self.conclude(result, State::Finished)
    }
    pub fn counters(&self) -> ChannelCounters {
        self.counters
    }
    pub fn udp_counters(&self) -> UdpCounters {
        self.peer.udp_counters()
    }
}

impl Drop for ObjectChannel {
    fn drop(&mut self) {
        if self.state != State::Finished {
            self.peer.abort();
        }
    }
}
