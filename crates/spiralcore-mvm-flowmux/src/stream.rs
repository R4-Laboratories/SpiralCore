use crate::{
    Cancellation, Duplex, DuplexReader, DuplexWriter, Error, Event, JoinGuard, MAX_BLOCK, Result,
    duplex::{deadline, lock_until},
};
use spiralcore_transfer::client::Stats;
use std::{
    io::{self, Read, Write},
    net::Shutdown,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

struct BufferedReader {
    reader: DuplexReader,
    bytes: Vec<u8>,
    offset: usize,
    ended: bool,
}
struct Shared {
    writer: Mutex<DuplexWriter>,
    reader: Mutex<BufferedReader>,
    guard: Mutex<Option<JoinGuard>>,
    cancellation: Cancellation,
    timeout: Duration,
}
impl Drop for Shared {
    fn drop(&mut self) {
        self.cancellation.cancel();
        let guard = self
            .guard
            .get_mut()
            .unwrap_or_else(|poison| poison.into_inner())
            .take();
        drop(guard);
    }
}
/// Relay-compatible payload I/O over one already selected cooperating connection.
#[derive(Clone)]
pub struct MappedStream {
    inner: Arc<Shared>,
}
impl std::fmt::Debug for MappedStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MappedStream").finish_non_exhaustive()
    }
}
impl MappedStream {
    pub fn from_duplex(duplex: Duplex) -> Self {
        let (cancellation, timeout) = duplex.stream_config();
        let (writer, reader, guard) = duplex.split();
        Self {
            inner: Arc::new(Shared {
                writer: Mutex::new(writer),
                reader: Mutex::new(BufferedReader {
                    reader,
                    bytes: Vec::new(),
                    offset: 0,
                    ended: false,
                }),
                guard: Mutex::new(Some(guard)),
                cancellation,
                timeout,
            }),
        }
    }
    fn write_operation<T>(
        &self,
        operation: impl FnOnce(&mut DuplexWriter, Instant) -> Result<T>,
    ) -> Result<T> {
        let until = deadline(self.inner.timeout)?;
        self.write_operation_until(until, operation)
    }
    fn write_operation_until<T>(
        &self,
        until: Instant,
        operation: impl FnOnce(&mut DuplexWriter, Instant) -> Result<T>,
    ) -> Result<T> {
        let result = (|| {
            let now = Instant::now();
            if until <= now {
                return Err(Error::Timeout);
            }
            if until.saturating_duration_since(now) > self.inner.timeout {
                return Err(Error::Limit);
            }
            self.inner.cancellation.check()?;
            let mut writer = lock_until(&self.inner.writer, until)?;
            operation(&mut writer, until)
        })();
        if result.is_err() && !matches!(&result, Err(Error::Closed)) {
            self.inner.cancellation.cancel();
        }
        result
    }
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        match how {
            Shutdown::Write => self
                .write_operation(|writer, until| writer.half_close_until(until))
                .map_err(io_error),
            Shutdown::Read | Shutdown::Both => {
                self.inner.cancellation.cancel();
                Ok(())
            }
        }
    }
    pub fn seed_base(&self, bytes: Vec<u8>) -> Result<Stats> {
        self.write_operation(|writer, until| writer.seed_base_until(bytes, until))
    }
    pub fn seed_base_until(&self, bytes: Vec<u8>, until: Instant) -> Result<Stats> {
        self.write_operation_until(until, |writer, until| writer.seed_base_until(bytes, until))
    }
    pub fn seed_schema_until(&self, until: Instant) -> Result<Stats> {
        self.write_operation_until(until, |writer, until| writer.seed_schema_until(until))
    }
    pub fn seed_checkpoint_until(&self, until: Instant) -> Result<Stats> {
        self.write_operation_until(until, |writer, until| writer.seed_checkpoint_until(until))
    }
    pub fn seed_schema(&self) -> Result<Stats> {
        self.write_operation(|writer, until| writer.seed_schema_until(until))
    }
    pub fn seed_checkpoint(&self) -> Result<Stats> {
        self.write_operation(|writer, until| writer.seed_checkpoint_until(until))
    }
}
fn io_error(error: Error) -> io::Error {
    let kind = match &error {
        Error::Io(error) => error.kind(),
        Error::Timeout => io::ErrorKind::TimedOut,
        Error::Closed | Error::Cancelled => io::ErrorKind::BrokenPipe,
        Error::InvalidBinding | Error::PeerMismatch | Error::Limit => io::ErrorKind::InvalidInput,
        Error::Protocol | Error::Sequence | Error::Integrity | Error::Truncated => {
            io::ErrorKind::InvalidData
        }
        Error::Core(_) | Error::Worker => io::ErrorKind::Other,
    };
    io::Error::new(kind, error)
}
impl Read for MappedStream {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        let result = (|| {
            self.inner.cancellation.check_delivery()?;
            let until = deadline(self.inner.timeout)?;
            let mut buffered = lock_until(&self.inner.reader, until)?;
            if buffered.ended {
                return Ok(0);
            }
            if buffered.offset == buffered.bytes.len() {
                match buffered.reader.receive_until(until)? {
                    Event::Data(bytes) => {
                        buffered.bytes = bytes;
                        buffered.offset = 0;
                    }
                    Event::HalfClose => {
                        buffered.ended = true;
                        return Ok(0);
                    }
                    Event::Reset | Event::Cancel => return Err(Error::Cancelled),
                }
            }
            let n = output.len().min(buffered.bytes.len() - buffered.offset);
            self.inner.cancellation.check_delivery()?;
            output[..n].copy_from_slice(&buffered.bytes[buffered.offset..buffered.offset + n]);
            buffered.offset += n;
            Ok(n)
        })();
        if result.is_err() {
            self.inner.cancellation.cancel();
        }
        result.map_err(io_error)
    }
}
impl Write for MappedStream {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        let n = bytes.len().min(MAX_BLOCK);
        self.write_operation(|writer, until| writer.send_until(bytes[..n].to_vec(), until))
            .map(|_| n)
            .map_err(io_error)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.cancellation.check().map_err(io_error)
    }
}
