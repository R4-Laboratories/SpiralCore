//! Explicitly owned development-witness caller; default calls remain strict.
use super::*;
use spiralcore_mvm_adapter::broker::BrokerContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TailDriveOutcome {
    NoTail,
    Idle,
    AckWritten,
}

pub struct CallerEndpoint<S: SocketIo> {
    client: Client<'static, S>,
}
impl<S: SocketIo + 'static> CallerEndpoint<S> {
    pub fn new(
        bootstrap: &Bootstrap,
        broker: &BrokerContext,
        socket: S,
        role: HandshakeRole,
    ) -> Result<Self> {
        if bootstrap.role != Role::Sender {
            return Err("sender bootstrap role mismatch".into());
        }
        let service = canonical::Session::new(bootstrap, broker)?;
        let route = bootstrap
            .outgoing
            .as_ref()
            .ok_or("missing outgoing material")?
            .binding
            .claims
            .route_id;
        let stream = StreamTransport::new(socket, route, CALL_TIMEOUT)?;
        let cancel = CancellationToken::default();
        stream.handshake(role, &cancel)?;
        Ok(Self {
            client: Client {
                stream,
                cancel,
                exchange: Meter::default(),
                service,
                on_call: None,
            },
        })
    }
    pub fn call(&mut self, request: &Frame) -> Result<(Context, Frame)> {
        self.client.call(request)
    }
    pub fn call_with_completed_tail(&mut self, request: &Frame) -> Result<(Context, Frame)> {
        self.client.service.call_with_completed_tail(
            request,
            &self.client.stream,
            &self.client.cancel,
            None,
        )
    }
    pub fn call_with_completed_tail_and_reconciliation(
        &mut self,
        request: &Frame,
        on_call: &mut impl FnMut(ReconciliationSignal) -> Result<()>,
    ) -> Result<(Context, Frame)> {
        let (signal, requests) = reconciliation::bounded_requests();
        on_call(signal)?;
        self.client.service.call_with_completed_tail(
            request,
            &self.client.stream,
            &self.client.cancel,
            Some(requests),
        )
    }
    pub fn drive_completed_tail(&mut self) -> Result<TailDriveOutcome> {
        self.client
            .service
            .drive_completed_tail(&self.client.stream, &self.client.cancel)
    }
    pub fn counters(&self) -> Result<StreamCounters> {
        Ok(self.client.stream.counters()?)
    }
    pub fn service_report(&self) -> Value {
        self.client.service.report()
    }
}
