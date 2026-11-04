//! Native host composition. Production requires canonical opaque admission.
use crate::*;
use mvm_contract::{
    grants::owner_access::AccessRefusal,
    protocol::owner_access::{
        OwnerAccessAudit, OwnerAccessAuditKind, OwnerAccessCallFrame, OwnerAccessReply,
        OwnerAccessReplyFrame, OwnerAccessRequest, OwnerAccessStatus, ScalarValue, TypedRecord,
    },
};
use mvm_hostd::{
    audit::emitter::AuditEmitter, plan_admission::AdmittedPlan,
    stream::access_guard::AccessPayloadGuard,
};
mod installed;
pub use installed::InstalledAccessFactory;
#[cfg(feature = "object-v1")]
mod object;
#[cfg(feature = "object-v1")]
pub use object::{ObjectOptions, ObjectReply};

struct CanonicalAudit {
    emitter: Arc<AuditEmitter>,
    plan: mvm_core::plan::ExecutionPlan,
}
impl Audit for CanonicalAudit {
    fn append(&mut self, record: &AuditRecord) -> Result<()> {
        use mvm_contract::grants::owner_access::{ExecutionIdentity, PolicyRevision};
        let kind = match record.kind {
            AuditKind::Granted => OwnerAccessAuditKind::Granted,
            AuditKind::Request => OwnerAccessAuditKind::Request,
            AuditKind::Refused => OwnerAccessAuditKind::Refused,
            AuditKind::Forward => OwnerAccessAuditKind::Forward,
            AuditKind::Completed => OwnerAccessAuditKind::Completed,
            AuditKind::Cancelled => OwnerAccessAuditKind::Cancelled,
            AuditKind::Revoked => OwnerAccessAuditKind::Revoked,
            AuditKind::Shutdown => OwnerAccessAuditKind::Shutdown,
        };
        let reason = record.reason.map(|reason| match reason {
            Error::Owner => AccessRefusal::Owner,
            Error::Expired => AccessRefusal::Expired,
            Error::Replay => AccessRefusal::Replay,
            Error::Limit | Error::Busy => AccessRefusal::Limit,
            Error::Epoch => AccessRefusal::Epoch,
            Error::Plan | Error::Signature => AccessRefusal::Identity,
            Error::Policy => AccessRefusal::Declaration,
            _ => AccessRefusal::Unavailable,
        });
        self.emitter
            .emit_owner_access(
                &self.plan,
                &OwnerAccessAudit {
                    kind,
                    policy: PolicyRevision {
                        plan_id: self.plan.plan_id.clone(),
                        plan_version: self.plan.plan_version,
                    },
                    execution: ExecutionIdentity {
                        instance_id: record.instance.clone(),
                        plan_nonce: self.plan.nonce.clone(),
                        bundle_sha256: self
                            .plan
                            .bundle
                            .as_ref()
                            .map(|bundle| bundle.bundle_sha256.clone()),
                    },
                    request_digest: record.request,
                    sequence: record.sequence,
                    operation: record.operation.clone(),
                    request_bytes: record.bytes as u64,
                    reply_bytes: 0,
                    reason,
                },
            )
            .map_err(|_| Error::Audit)
    }
}
pub struct NativeAccess {
    service: AccessService<CanonicalAudit>,
    input: AccessPayloadGuard,
    output: AccessPayloadGuard,
    #[cfg(feature = "object-v1")]
    object_root: Option<object::StoreRoot>,
}
#[derive(Debug, PartialEq, Eq)]
pub struct DevReply {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: i32,
}
pub struct DevCall<'a> {
    pub id: [u8; 32],
    pub command: &'a str,
    pub stdin: Option<&'a str>,
    pub deadline_unix_ms: u64,
}
impl NativeAccess {
    /// The opaque admitted guard carries canonical host policy, artifacts,
    /// replay admission and budget decisions. Caller-supplied keys cannot
    /// replace it, and no payload or signature-only plan can construct it.
    pub fn from_admitted(
        admitted: &AdmittedPlan,
        input: Admission<'_>,
        emitter: Arc<AuditEmitter>,
    ) -> Result<Self> {
        if input.signed.0.payload != admitted.signed().0.payload
            || input.signed.0.signature != admitted.signed().0.signature
            || input.signed.0.signer_id != admitted.signed().0.signer_id
        {
            return Err(Error::Plan);
        }
        Self::from_plan(admitted.plan().clone(), input, emitter)
    }
    #[cfg(test)]
    pub(super) fn component_fixture(
        input: Admission<'_>,
        emitter: Arc<AuditEmitter>,
    ) -> Result<Self> {
        let plan = verify_plan(input.signed, input.keys).map_err(|_| Error::Signature)?;
        Self::from_plan(plan, input, emitter)
    }
    fn from_plan(
        plan: mvm_core::plan::ExecutionPlan,
        input: Admission<'_>,
        emitter: Arc<AuditEmitter>,
    ) -> Result<Self> {
        let binding = input.control.snapshot()?;
        let inward = AccessPayloadGuard::for_instance(&plan, &binding.instance)
            .map_err(|_| Error::Policy)?;
        let outward = AccessPayloadGuard::for_instance(&plan, &binding.instance)
            .map_err(|_| Error::Policy)?;
        let audit = CanonicalAudit {
            emitter,
            plan: plan.clone(),
        };
        let service = AccessService::admit_verified(plan, input, audit)?;
        Ok(Self {
            service,
            input: inward,
            output: outward,
            #[cfg(feature = "object-v1")]
            object_root: None,
        })
    }
    pub fn counters(&self) -> Counters {
        self.service.counters()
    }
    pub fn call_dev<S: AccessSocket>(
        &mut self,
        client: &mut OwnerClient<S>,
        owner: &str,
        id: [u8; 32],
        command: &str,
        stdin: Option<&str>,
    ) -> Result<DevReply> {
        let bytes = command
            .len()
            .checked_add(stdin.map_or(0, str::len))
            .ok_or(Error::Limit)?;
        let handle = self.service.begin(owner, "exec", id, bytes)?;
        self.dev_exchange(client, owner, command, stdin, handle)
    }
    fn dev_exchange<S: AccessSocket>(
        &mut self,
        client: &mut OwnerClient<S>,
        owner: &str,
        command: &str,
        stdin: Option<&str>,
        handle: RequestHandle,
    ) -> Result<DevReply> {
        use mvm_agentd::vsock::{ExecEvent, GuestRequest, GuestResponse};
        let bytes = command
            .len()
            .checked_add(stdin.map_or(0, str::len))
            .ok_or(Error::Limit)?;
        let mut retained = Some(handle);
        let result = (|| {
            let handle = retained.as_ref().ok_or(Error::State)?;
            let pending = self.service.pending(handle)?;
            let guard = crate::transport::IoGuard {
                control: self.service.control.clone(),
                binding: self.service.binding.clone(),
                clock: self.service.clock.clone(),
                deadline: pending.deadline,
                unix_deadline: pending.unix_deadline,
                extra_current: None,
            };
            let command = String::from_utf8(
                self.input
                    .clear_message(command.as_bytes())
                    .map_err(|_| Error::Policy)?,
            )
            .map_err(|_| Error::Policy)?;
            let stdin = stdin
                .map(|input| {
                    self.input
                        .clear_message(input.as_bytes())
                        .map_err(|_| Error::Policy)
                        .and_then(|bytes| String::from_utf8(bytes).map_err(|_| Error::Policy))
                })
                .transpose()?;
            let request = GuestRequest::Exec {
                command,
                stdin,
                timeout_secs: None,
                deadline_unix_ms: Some(
                    u64::try_from(guard.unix_deadline.timestamp_millis())
                        .map_err(|_| Error::Expired)?,
                ),
            };
            self.service
                .forward(handle, owner, &guard.binding, bytes, || {
                    client.write_message(&request, guard.deadline, Some(&guard))
                })?;
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let code = loop {
                let reply: GuestResponse =
                    client.read_message(guard.deadline, Some(&guard), 65536 + 4096)?;
                self.service
                    .forward(handle, owner, &guard.binding, 0, || Ok(()))?;
                match reply {
                    GuestResponse::ExecEvent(ExecEvent::Stdout { chunk }) => stdout.extend(chunk),
                    GuestResponse::ExecEvent(ExecEvent::Stderr { chunk }) => stderr.extend(chunk),
                    GuestResponse::ExecEvent(ExecEvent::Exit { code }) => break code,
                    GuestResponse::ExecEvent(ExecEvent::TimedOut) => return Err(Error::Expired),
                    _ => return Err(Error::Policy),
                }
                if stdout
                    .len()
                    .checked_add(stderr.len())
                    .is_none_or(|size| size > 65536)
                {
                    return Err(Error::Limit);
                }
            };
            // Withhold the entire bounded result until both complete streams
            // have passed the existing secret scanner and policy transform.
            let stdout = self
                .output
                .clear_message(&stdout)
                .map_err(|_| Error::Policy)?;
            let stderr = self
                .output
                .clear_message(&stderr)
                .map_err(|_| Error::Policy)?;
            self.service.forward(
                handle,
                owner,
                &guard.binding,
                stdout.len() + stderr.len(),
                || Ok(()),
            )?;
            self.service
                .complete(retained.take().ok_or(Error::State)?)?;
            Ok(DevReply {
                stdout,
                stderr,
                code,
            })
        })();
        if result.is_err() {
            if let Some(handle) = retained.take() {
                let _ = self.service.cancel(handle);
            }
            client.close();
        }
        result
    }
    pub fn call_typed<S: AccessSocket>(
        &mut self,
        client: &mut OwnerClient<S>,
        owner: &str,
        request: OwnerAccessRequest,
    ) -> Result<OwnerAccessReply> {
        if self
            .service
            .grant
            .owner_access
            .as_ref()
            .is_some_and(|binding| binding.grant.object_v1.is_some())
        {
            #[cfg(feature = "object-v1")]
            {
                let root = self.object_root.clone().ok_or(Error::Unsupported)?;
                return self
                    .call_typed_object_at(client, owner, request, &root, ObjectOptions::default())
                    .map(|reply| reply.reply);
            }
            #[cfg(not(feature = "object-v1"))]
            return Err(Error::Unsupported);
        }
        let handle = self.service.begin_typed(owner, &request)?;
        self.typed_exchange(client, owner, request, handle)
    }
    fn typed_exchange<S: AccessSocket>(
        &mut self,
        client: &mut OwnerClient<S>,
        owner: &str,
        mut request: OwnerAccessRequest,
        handle: RequestHandle,
    ) -> Result<OwnerAccessReply> {
        let mut retained = Some(handle);
        let result = (|| {
            let handle = retained.as_ref().ok_or(Error::State)?;
            let operation = self
                .service
                .grant
                .owner_access
                .as_ref()
                .ok_or(Error::Unsupported)?
                .grant
                .operation(&request.operation)
                .map_err(|_| Error::Policy)?
                .clone();
            clear_record(&mut self.input, &mut request.payload)?;
            operation
                .validate_request(&request.payload)
                .map_err(|_| Error::Policy)?;
            let pending = self.service.pending(handle)?;
            let guard = crate::transport::IoGuard {
                control: self.service.control.clone(),
                binding: self.service.binding.clone(),
                clock: self.service.clock.clone(),
                deadline: pending.deadline,
                unix_deadline: pending.unix_deadline,
                extra_current: None,
            };
            let bytes = serde_json::to_vec(&request.payload)
                .map_err(|_| Error::Policy)?
                .len();
            if bytes > pending._charge.bytes {
                return Err(Error::Limit);
            }
            let frame = OwnerAccessCallFrame::new(request.clone());
            self.service
                .forward(handle, owner, &guard.binding, bytes, || {
                    client.write_message(&frame, guard.deadline, Some(&guard))
                })?;
            let OwnerAccessReplyFrame { mut reply } = client.read_message(
                guard.deadline,
                Some(&guard),
                operation.max_reply_bytes as usize + 4096,
            )?;
            if reply.request_id != request.request_id || reply.sequence != request.sequence {
                return Err(Error::Policy);
            }
            if reply.status == OwnerAccessStatus::Ok {
                operation
                    .validate_reply(&reply.payload)
                    .map_err(|_| Error::Policy)?;
            } else if !reply.payload.fields.is_empty() {
                return Err(Error::Policy);
            }
            clear_record(&mut self.output, &mut reply.payload)?;
            if reply.status == OwnerAccessStatus::Ok {
                operation
                    .validate_reply(&reply.payload)
                    .map_err(|_| Error::Policy)?;
            }
            let bytes = serde_json::to_vec(&reply.payload)
                .map_err(|_| Error::Policy)?
                .len();
            self.service
                .forward(handle, owner, &guard.binding, bytes, || Ok(()))?;
            let handle = retained.take().ok_or(Error::State)?;
            if reply.status == OwnerAccessStatus::Ok {
                self.service.complete(handle)?;
            } else {
                self.service.cancel(handle)?;
            }
            Ok(reply)
        })();
        if result.is_err() {
            if let Some(handle) = retained.take() {
                let _ = self.service.cancel(handle);
            }
            client.close();
        }
        result
    }
    fn connect_pending<S: AccessSocket>(
        &mut self,
        handle: &RequestHandle,
        connect: impl FnOnce(
            Instant,
        )
            -> Result<(S, ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey)>,
    ) -> Result<OwnerClient<S>> {
        let pending = self.service.pending(handle)?;
        let guard = crate::transport::IoGuard {
            control: self.service.control.clone(),
            binding: self.service.binding.clone(),
            clock: self.service.clock.clone(),
            deadline: pending.deadline,
            unix_deadline: pending.unix_deadline,
            extra_current: None,
        };
        guard.control.snapshot()?;
        let (socket, key, peer) = connect(guard.deadline)?;
        OwnerClient::connect_guarded(socket, &guard.binding.instance, key, &peer, &guard)
    }
    /// Starts admission before any connect, handshake or discovery. All later
    /// phases share this exact request handle and its original two clock caps.
    pub fn call_typed_connect<S: AccessSocket>(
        &mut self,
        owner: &str,
        request: OwnerAccessRequest,
        connect: impl FnOnce(
            Instant,
        )
            -> Result<(S, ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey)>,
    ) -> Result<OwnerAccessReply> {
        if self
            .service
            .grant
            .owner_access
            .as_ref()
            .is_some_and(|binding| binding.grant.object_v1.is_some())
        {
            #[cfg(feature = "object-v1")]
            {
                let root = self.object_root.clone().ok_or(Error::Unsupported)?;
                return self
                    .call_typed_object_connect_at(
                        owner,
                        request,
                        &root,
                        ObjectOptions::default(),
                        connect,
                    )
                    .map(|reply| reply.reply);
            }
            #[cfg(not(feature = "object-v1"))]
            return Err(Error::Unsupported);
        }
        let handle = self.service.begin_typed(owner, &request)?;
        match self.connect_pending(&handle, connect) {
            Ok(mut client) => self.typed_exchange(&mut client, owner, request, handle),
            Err(error) => {
                let _ = self.service.cancel(handle);
                Err(error)
            }
        }
    }
    pub fn call_dev_connect<S: AccessSocket>(
        &mut self,
        owner: &str,
        call: DevCall<'_>,
        connect: impl FnOnce(
            Instant,
        )
            -> Result<(S, ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey)>,
    ) -> Result<DevReply> {
        let bytes = call
            .command
            .len()
            .checked_add(call.stdin.map_or(0, str::len))
            .ok_or(Error::Limit)?;
        let handle = self.service.begin(owner, "exec", call.id, bytes)?;
        let restricted = (|| {
            let now = self.service.current()?;
            let unix = DateTime::from_timestamp_millis(
                i64::try_from(call.deadline_unix_ms).map_err(|_| Error::Expired)?,
            )
            .ok_or(Error::Expired)?;
            let remaining = (unix - now.unix).to_std().map_err(|_| Error::Expired)?;
            if remaining.is_zero() || remaining > Duration::from_secs(30) {
                return Err(Error::Expired);
            }
            let pending = self
                .service
                .pending
                .get_mut(&handle.id)
                .ok_or(Error::State)?;
            pending.unix_deadline = pending.unix_deadline.min(unix);
            pending.deadline = pending
                .deadline
                .min(now.monotonic.checked_add(remaining).ok_or(Error::Expired)?);
            Ok(())
        })();
        if let Err(error) = restricted {
            let _ = self.service.cancel(handle);
            return Err(error);
        }
        match self.connect_pending(&handle, connect) {
            Ok(mut client) => {
                self.dev_exchange(&mut client, owner, call.command, call.stdin, handle)
            }
            Err(error) => {
                let _ = self.service.cancel(handle);
                Err(error)
            }
        }
    }
}
fn clear_record(guard: &mut AccessPayloadGuard, record: &mut TypedRecord) -> Result<()> {
    // Inspect semantic byte/text fields before JSON encoding can disguise a
    // known secret as escaping or a decimal byte array. Revalidate afterwards.
    for field in &mut record.fields {
        match &mut field.value {
            ScalarValue::Text(text) => {
                let bytes = guard
                    .clear_message(text.as_bytes())
                    .map_err(|_| Error::Policy)?;
                *text = String::from_utf8(bytes).map_err(|_| Error::Policy)?;
            }
            ScalarValue::Bytes(bytes) => {
                *bytes = guard.clear_message(bytes).map_err(|_| Error::Policy)?
            }
            _ => {}
        }
    }
    let encoded = serde_json::to_vec(record).map_err(|_| Error::Policy)?;
    let cleared = guard.clear_message(&encoded).map_err(|_| Error::Policy)?;
    *record = serde_json::from_slice(&cleared).map_err(|_| Error::Policy)?;
    Ok(())
}
