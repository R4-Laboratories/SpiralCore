//! The canonical object mapping uses the same shared core as mailbox/FlowMux.
//! Options and store roots are trusted host configuration, never guest argv,
//! application selectors, remotely supplied filesystem paths or permissions.
use super::*;
use mvm_agentd::{
    entrypoint::CancellationToken,
    owner_access_object::{AccessObjectScope, ObjectMode},
};
use mvm_contract::protocol::owner_access::{
    OwnerAccessObjectBegin, OwnerAccessObjectCallFrame, OwnerAccessObjectDirection as D,
    OwnerAccessObjectFrames, OwnerAccessObjectReplyFrame, OwnerAccessObjectRequest as Q,
    OwnerAccessObjectResponse as R, join_object_payload, split_object_payload,
};
use spiralcore_transfer::{
    protocol::Frame, resource_profile::ResourceProfile, shared_transfer::PreparedPrediction,
    v1::ObjectReceipt,
};
use std::path::Path;

/// Borrowed preparation inputs remain charged to their external owner. Copies,
/// prepared codecs and retained history constructed here use the shared lease.
pub struct ObjectOptions<'a> {
    pub mode: ObjectMode,
    pub schema: Option<&'a [u8]>,
    pub prediction: Option<(&'a [u8], &'a spiralcore_transfer::shared_runtime::Recipe)>,
}
impl Default for ObjectOptions<'_> {
    fn default() -> Self {
        Self {
            mode: ObjectMode::Whole,
            schema: None,
            prediction: None,
        }
    }
}
impl ObjectOptions<'_> {
    /// Four bounded frame/JSON/crypto workspaces per end, two ends. This is
    /// additional to shared-core and retained application payload reservations.
    pub fn wire_charge() -> usize {
        WIRE_RESERVATION
    }
}
#[derive(Debug)]
pub struct ObjectReply {
    pub reply: OwnerAccessReply,
    pub input_receipt: ObjectReceipt,
    /// Actual encoded shared-core frames carried by successful input messages,
    /// including acknowledged schema/checkpoint seeds. Carrier totals below
    /// additionally include canonical JSON, cryptography and control messages.
    pub input_core_sent: u64,
    pub input_core_received: u64,
    /// Actual encrypted object-phase wire I/O. Discovery/setup precedes this
    /// interval and remains in OwnerClient's cumulative counters.
    pub wire: WireCounters,
    pub elapsed_micros: u128,
    pub codec_reservation: u64,
    pub wire_reservation: usize,
}
pub(super) const WIRE_RESERVATION: usize = 8 * (256 * 1024 + 8192);
#[derive(Clone)]
pub(super) struct StoreRoot(Arc<StoreRootState>);
struct StoreRootState {
    path: std::path::PathBuf,
    _file: std::fs::File,
    identity: (u64, u64),
    fenced: std::sync::atomic::AtomicBool,
}
impl StoreRoot {
    pub(super) fn open(path: std::path::PathBuf) -> Result<Self> {
        use std::os::unix::fs::MetadataExt;
        validate_store_root(&path)?;
        let file =
            mvm_vmm::host::restore_image::open_trusted_dir(&path).map_err(|_| Error::Policy)?;
        let metadata = file.metadata().map_err(|_| Error::Policy)?;
        let root = Self(Arc::new(StoreRootState {
            path,
            _file: file,
            identity: (metadata.dev(), metadata.ino()),
            fenced: std::sync::atomic::AtomicBool::new(false),
        }));
        root.check()?;
        Ok(root)
    }
    pub(super) fn path(&self) -> &Path {
        &self.0.path
    }
    pub(super) fn check(&self) -> Result<()> {
        use std::os::unix::fs::MetadataExt;
        if self.0.fenced.load(Ordering::Acquire) {
            return Err(Error::Closed);
        }
        let result = (|| {
            validate_store_root(self.path())?;
            let current = mvm_vmm::host::restore_image::open_trusted_dir(self.path())
                .map_err(|_| Error::Policy)?;
            let metadata = current.metadata().map_err(|_| Error::Policy)?;
            if (metadata.dev(), metadata.ino()) != self.0.identity {
                return Err(Error::Policy);
            }
            Ok(())
        })();
        if result.is_err() {
            self.0.fenced.store(true, Ordering::Release);
        }
        result
    }
}
pub(super) fn validate_store_root(root: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if !root.is_absolute()
        || root.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err(Error::Policy);
    }
    let metadata = std::fs::symlink_metadata(root).map_err(|_| Error::Policy)?;
    mvm_core::private_fs::require_current_owner(&metadata).map_err(|_| Error::Policy)?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.permissions().mode() & 0o077 != 0
        || root.canonicalize().map_err(|_| Error::Policy)? != root
    {
        return Err(Error::Policy);
    }
    Ok(())
}
fn owned_store(root: &StoreRoot) -> Result<tempfile::TempDir> {
    use std::os::unix::fs::PermissionsExt;
    // The caller supplies an already authorized private host root. The guest
    // never supplies this path. Only this fresh owned child is removed on Drop.
    root.check()?;
    let store = tempfile::Builder::new()
        .prefix("access-call-")
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir_in(root.path())
        .map_err(|_| Error::Policy)?;
    root.check()?;
    Ok(store)
}
fn core_error(error: spiralcore_transfer::Error) -> Error {
    match error {
        spiralcore_transfer::Error::Limit => Error::Limit,
        spiralcore_transfer::Error::Expired => Error::Expired,
        spiralcore_transfer::Error::Unsupported => Error::Unsupported,
        _ => Error::Policy,
    }
}
struct ObjectWire<'a, S: AccessSocket> {
    service: &'a mut AccessService<CanonicalAudit>,
    handle: &'a RequestHandle,
    owner: &'a str,
    client: &'a mut OwnerClient<S>,
    guard: crate::transport::IoGuard,
    begin: &'a OwnerAccessObjectBegin,
    last_error: Option<Error>,
    input_core_sent: u64,
    input_core_received: u64,
}
struct ObjectExchange<'a> {
    owner: &'a str,
    request: OwnerAccessRequest,
    root: &'a Path,
    options: ObjectOptions<'a>,
    handle: RequestHandle,
    resources: (ObjectCharge, ObjectCharge, u64),
    storage: StoreRoot,
}
impl<S: AccessSocket> ObjectWire<'_, S> {
    fn send(&mut self, request: Q, bytes: usize) -> Result<()> {
        self.service
            .forward(self.handle, self.owner, &self.guard.binding, bytes, || {
                self.client.write_message(
                    &OwnerAccessObjectCallFrame { request },
                    self.guard.deadline,
                    Some(&self.guard),
                )
            })
    }
    fn receive(&mut self, bytes: usize) -> Result<R> {
        let mut reply = None;
        self.service
            .forward(self.handle, self.owner, &self.guard.binding, bytes, || {
                reply = Some(
                    self.client
                        .read_message::<OwnerAccessObjectReplyFrame>(
                            self.guard.deadline,
                            Some(&self.guard),
                            256 * 1024 + 8192,
                        )?
                        .reply,
                );
                Ok(())
            })?;
        reply.ok_or(Error::State)
    }
    fn current(&self, id: [u8; 32], sequence: u64) -> Result<()> {
        if id != self.begin.request.request_id || sequence != self.begin.request.sequence {
            return Err(Error::Policy);
        }
        Ok(())
    }
    fn input(&mut self, frames: &[Frame]) -> spiralcore_transfer::Result<Vec<Frame>> {
        let result = (|| {
            let mut replies = Vec::with_capacity(frames.len());
            for frame in frames {
                let bytes = frame.encode().map_err(core_error)?;
                let encoded_len = bytes.len() as u64;
                self.send(
                    Q::Frames(OwnerAccessObjectFrames {
                        request_id: self.begin.request.request_id,
                        sequence: self.begin.request.sequence,
                        direction: D::Input,
                        frames: vec![bytes],
                    }),
                    0,
                )?;
                self.input_core_sent = self
                    .input_core_sent
                    .checked_add(encoded_len)
                    .ok_or(Error::Limit)?;
                let R::Frames(response) = self.receive(0)? else {
                    return Err(Error::Policy);
                };
                response
                    .validate_for(self.begin, D::Input)
                    .map_err(|_| Error::Policy)?;
                for bytes in &response.frames {
                    self.input_core_received = self
                        .input_core_received
                        .checked_add(bytes.len() as u64)
                        .ok_or(Error::Limit)?;
                    replies.push(Frame::decode(bytes).map_err(core_error)?);
                }
                if replies.last().is_some_and(|reply| reply.kind == 103) {
                    break;
                }
            }
            Ok(replies)
        })();
        result.map_err(|error| {
            self.last_error = Some(error);
            spiralcore_transfer::Error::Unauthorized
        })
    }
}
impl NativeAccess {
    pub fn with_object_store(mut self, root: std::path::PathBuf) -> Result<Self> {
        self.object_root = Some(StoreRoot::open(root)?);
        Ok(self)
    }
    pub fn call_typed_object<S: AccessSocket>(
        &mut self,
        client: &mut OwnerClient<S>,
        owner: &str,
        request: OwnerAccessRequest,
        root: &Path,
        options: ObjectOptions<'_>,
    ) -> Result<ObjectReply> {
        let root = StoreRoot::open(root.to_path_buf())?;
        self.call_typed_object_at(client, owner, request, &root, options)
    }
    pub(super) fn call_typed_object_at<S: AccessSocket>(
        &mut self,
        client: &mut OwnerClient<S>,
        owner: &str,
        request: OwnerAccessRequest,
        root: &StoreRoot,
        options: ObjectOptions<'_>,
    ) -> Result<ObjectReply> {
        if !client.object_negotiated() {
            return Err(Error::Unsupported);
        }
        let handle = self.service.begin_typed(owner, &request)?;
        let resources = match self.object_resources() {
            Ok(resources) => resources,
            Err(error) => {
                let _ = self.service.cancel(handle);
                return Err(error);
            }
        };
        let store = match owned_store(root) {
            Ok(store) => store,
            Err(error) => {
                let _ = self.service.cancel(handle);
                return Err(error);
            }
        };
        self.typed_object_exchange(
            client,
            ObjectExchange {
                owner,
                request,
                root: &store.path().join("receiver"),
                options,
                handle,
                resources,
                storage: root.clone(),
            },
        )
    }
    fn object_resources(&self) -> Result<(ObjectCharge, ObjectCharge, u64)> {
        let charge = ResourceProfile::SmallV1
            .operation_charge()
            .map_err(core_error)?;
        let wire = self.service.budget.reserve_object(0, WIRE_RESERVATION)?;
        let pair = self
            .service
            .budget
            .reserve_object(charge.try_into().map_err(|_| Error::Limit)?, 0)?;
        Ok((wire, pair, charge))
    }
    pub fn call_typed_object_connect<S: AccessSocket>(
        &mut self,
        owner: &str,
        request: OwnerAccessRequest,
        root: &Path,
        options: ObjectOptions<'_>,
        connect: impl FnOnce(
            Instant,
        )
            -> Result<(S, ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey)>,
    ) -> Result<ObjectReply> {
        let root = StoreRoot::open(root.to_path_buf())?;
        self.call_typed_object_connect_at(owner, request, &root, options, connect)
    }
    pub(super) fn call_typed_object_connect_at<S: AccessSocket>(
        &mut self,
        owner: &str,
        request: OwnerAccessRequest,
        root: &StoreRoot,
        options: ObjectOptions<'_>,
        connect: impl FnOnce(
            Instant,
        )
            -> Result<(S, ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey)>,
    ) -> Result<ObjectReply> {
        let handle = self.service.begin_typed(owner, &request)?;
        let resources = match self.object_resources() {
            Ok(resources) => resources,
            Err(error) => {
                let _ = self.service.cancel(handle);
                return Err(error);
            }
        };
        let store = match owned_store(root) {
            Ok(store) => store,
            Err(error) => {
                let _ = self.service.cancel(handle);
                return Err(error);
            }
        };
        let pending = self.service.pending(&handle)?;
        let storage_current = root.clone();
        let guard = crate::transport::IoGuard {
            control: self.service.control.clone(),
            binding: self.service.binding.clone(),
            clock: self.service.clock.clone(),
            deadline: pending.deadline,
            unix_deadline: pending.unix_deadline,
            extra_current: Some(Arc::new(move || storage_current.check())),
        };
        let connected = (|| {
            guard.check()?;
            let (socket, key, peer) = connect(guard.deadline)?;
            OwnerClient::connect_object_guarded(socket, &guard.binding.instance, key, &peer, &guard)
        })();
        match connected {
            Ok(mut client) => self.typed_object_exchange(
                &mut client,
                ObjectExchange {
                    owner,
                    request,
                    root: &store.path().join("receiver"),
                    options,
                    handle,
                    resources,
                    storage: root.clone(),
                },
            ),
            Err(error) => {
                let _ = self.service.cancel(handle);
                Err(error)
            }
        }
    }
    fn typed_object_exchange<S: AccessSocket>(
        &mut self,
        client: &mut OwnerClient<S>,
        call: ObjectExchange<'_>,
    ) -> Result<ObjectReply> {
        let ObjectExchange {
            owner,
            mut request,
            root,
            options,
            handle,
            resources,
            storage,
        } = call;
        let (_wire_resources, resources, charge) = resources;
        let mut retained = Some(handle);
        let result = (|| {
            let started = Instant::now();
            let before = client.wire_counters();
            let handle = retained.as_ref().ok_or(Error::State)?;
            let binding = self
                .service
                .grant
                .owner_access
                .clone()
                .ok_or(Error::Unsupported)?;
            let operation = binding
                .grant
                .operation(&request.operation)
                .map_err(|_| Error::Policy)?
                .clone();
            clear_record(&mut self.input, &mut request.payload)?;
            operation
                .validate_request(&request.payload)
                .map_err(|_| Error::Policy)?;
            let pending = self.service.pending(handle)?;
            let storage_current = storage.clone();
            let guard = crate::transport::IoGuard {
                control: self.service.control.clone(),
                binding: self.service.binding.clone(),
                clock: self.service.clock.clone(),
                deadline: pending.deadline,
                unix_deadline: pending.unix_deadline,
                extra_current: Some(Arc::new(move || storage_current.check())),
            };
            let token = CancellationToken::default();
            // The original request owns its decoded plaintext reservation.
            let (metadata, field, bytes) =
                split_object_payload(request.payload).map_err(|_| Error::Policy)?;
            request.payload = metadata;
            let begin = OwnerAccessObjectBegin {
                request,
                object: spiralcore_transfer::object_id(&bytes),
                payload_bytes: bytes.len().try_into().map_err(|_| Error::Limit)?,
                field,
            };
            begin
                .validate(
                    &binding,
                    u64::try_from(self.service.clock.now().unix.timestamp_millis())
                        .map_err(|_| Error::Expired)?,
                )
                .map_err(|_| Error::Policy)?;
            let input = AccessObjectScope::new(
                &binding,
                &begin.request,
                &binding.tenant,
                D::Input,
                token.clone(),
                guard.deadline,
            )
            .map_err(core_error)?;
            let prediction = options
                .prediction
                .map(|(checkpoint, recipe)| {
                    PreparedPrediction::new_with_resource_profile(
                        checkpoint,
                        recipe,
                        ResourceProfile::SmallV1,
                    )
                })
                .transpose()
                .map_err(core_error)?;
            let mut sender = input
                .sender(options.mode, options.schema, prediction)
                .map_err(core_error)?;
            let mut wire = ObjectWire {
                service: &mut self.service,
                handle,
                owner,
                client,
                guard,
                begin: &begin,
                last_error: None,
                input_core_sent: 0,
                input_core_received: 0,
            };
            wire.send(Q::Begin(Box::new(begin.clone())), bytes.len())?;
            match wire.receive(0)? {
                R::Ready {
                    request_id,
                    sequence,
                } => wire.current(request_id, sequence)?,
                _ => return Err(Error::Policy),
            }
            if matches!(options.mode, ObjectMode::Runtime | ObjectMode::Combined) {
                let transfer = spiralcore_transfer::hash(
                    b"mvm.owner-access.checkpoint-transfer.v1\0",
                    &[&begin.request.request_id],
                );
                let result =
                    input.seed_checkpoint(&mut sender, transfer, |frames| wire.input(frames));
                if let Err(error) = result {
                    return Err(wire.last_error.take().unwrap_or_else(|| core_error(error)));
                }
            }
            if matches!(options.mode, ObjectMode::Warm | ObjectMode::Combined) {
                let transfer = spiralcore_transfer::hash(
                    b"mvm.owner-access.schema-transfer.v1\0",
                    &[&begin.request.request_id],
                );
                let result = input.seed_schema(&mut sender, transfer, |frames| wire.input(frames));
                if let Err(error) = result {
                    return Err(wire.last_error.take().unwrap_or_else(|| core_error(error)));
                }
            }
            let transfer = spiralcore_transfer::hash(
                b"mvm.owner-access.input-transfer.v1\0",
                &[&begin.request.request_id],
            );
            let input_receipt = input
                .send(&mut sender, bytes, transfer, |frames| wire.input(frames))
                .map_err(|error| wire.last_error.take().unwrap_or_else(|| core_error(error)))?;
            drop(sender);
            drop(input);
            drop(resources);
            wire.send(
                Q::Commit {
                    request_id: begin.request.request_id,
                    sequence: begin.request.sequence,
                },
                0,
            )?;
            let R::Reply {
                request_id,
                sequence,
                status,
                object,
                payload_bytes,
                record,
                field,
            } = wire.receive(0)?
            else {
                return Err(Error::Policy);
            };
            wire.current(request_id, sequence)?;
            if payload_bytes == 0
                || payload_bytes as usize > 16 * 1024
                || payload_bytes > operation.max_reply_bytes
            {
                return Err(Error::Limit);
            }
            let output = AccessObjectScope::new(
                &binding,
                &begin.request,
                &binding.tenant,
                D::Reply,
                token,
                wire.guard.deadline,
            )
            .map_err(core_error)?;
            let _resources = wire
                .service
                .budget
                .reserve_object(charge.try_into().map_err(|_| Error::Limit)?, 0)?;
            storage.check()?;
            let mut receiver = output.receiver(root).map_err(core_error)?;
            wire.send(
                Q::ReplyReady {
                    request_id,
                    sequence,
                },
                0,
            )?;
            loop {
                let response = wire.receive(0)?;
                match response {
                    R::ReplyFrames(frames) => {
                        frames
                            .validate_for(&begin, D::Reply)
                            .map_err(|_| Error::Policy)?;
                        let frames = frames
                            .frames
                            .iter()
                            .map(|bytes| Frame::decode(bytes).map_err(core_error))
                            .collect::<Result<Vec<_>>>()?;
                        let replies = receiver.handle(&frames).map_err(core_error)?;
                        wire.send(
                            Q::Receipt(OwnerAccessObjectFrames {
                                request_id,
                                sequence,
                                direction: D::Reply,
                                frames: replies
                                    .iter()
                                    .map(Frame::encode)
                                    .collect::<spiralcore_transfer::Result<Vec<_>>>()
                                    .map_err(core_error)?,
                            }),
                            0,
                        )?;
                    }
                    R::Completed {
                        request_id: observed,
                        sequence: observed_sequence,
                        status: observed_status,
                        object: observed_object,
                        payload_bytes: observed_bytes,
                    } => {
                        wire.current(observed, observed_sequence)?;
                        if observed_status != status
                            || observed_object != object
                            || observed_bytes != payload_bytes
                        {
                            return Err(Error::Policy);
                        }
                        break;
                    }
                    _ => return Err(Error::Policy),
                }
            }
            storage.check()?;
            let bytes = receiver
                .read_object(object, payload_bytes as usize)
                .map_err(core_error)?;
            storage.check()?;
            let mut payload = join_object_payload(&record, field.as_deref(), &bytes)
                .map_err(|_| Error::Policy)?;
            drop(receiver);
            drop(output);
            drop(_resources);
            drop(bytes);
            if status == OwnerAccessStatus::Ok {
                operation
                    .validate_reply(&payload)
                    .map_err(|_| Error::Policy)?;
            } else if !payload.fields.is_empty() {
                return Err(Error::Policy);
            }
            clear_record(&mut self.output, &mut payload)?;
            if status == OwnerAccessStatus::Ok {
                operation
                    .validate_reply(&payload)
                    .map_err(|_| Error::Policy)?;
            }
            let after = wire.client.wire_counters();
            let wire_counts = WireCounters {
                sent: after.sent.checked_sub(before.sent).ok_or(Error::State)?,
                received: after
                    .received
                    .checked_sub(before.received)
                    .ok_or(Error::State)?,
                frames_sent: after
                    .frames_sent
                    .checked_sub(before.frames_sent)
                    .ok_or(Error::State)?,
                frames_received: after
                    .frames_received
                    .checked_sub(before.frames_received)
                    .ok_or(Error::State)?,
            };
            let input_core_sent = wire.input_core_sent;
            let input_core_received = wire.input_core_received;
            drop(wire);
            let handle = retained.take().ok_or(Error::State)?;
            if status == OwnerAccessStatus::Ok {
                self.service.complete(handle)?;
            } else {
                self.service.cancel(handle)?;
            }
            Ok(ObjectReply {
                reply: OwnerAccessReply {
                    request_id,
                    sequence,
                    status,
                    payload,
                },
                input_receipt,
                input_core_sent,
                input_core_received,
                wire: wire_counts,
                elapsed_micros: started.elapsed().as_micros(),
                codec_reservation: charge,
                wire_reservation: WIRE_RESERVATION,
            })
        })();
        if result.is_err() {
            client.close();
            if let Some(handle) = retained {
                let _ = self.service.cancel(handle);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    #[test]
    fn codec_and_carrier_reservations_share_one_budget_and_default_to_refusal() {
        let disabled = SharedBudget::new(Limits::default()).unwrap();
        assert!(matches!(disabled.reserve_object(1, 1), Err(Error::Busy)));
        assert_eq!(disabled.object_usage().unwrap(), (0, 0));
        let codec = ResourceProfile::SmallV1.operation_charge().unwrap() as usize;
        let budget = SharedBudget::new(Limits {
            codec_bytes: codec,
            wire_bytes: WIRE_RESERVATION,
            ..Default::default()
        })
        .unwrap();
        let peer = budget.clone();
        let first = budget.reserve_object(codec, WIRE_RESERVATION).unwrap();
        assert!(matches!(peer.reserve_object(1, 0), Err(Error::Busy)));
        assert!(matches!(
            peer.reserve_object(usize::MAX, 0),
            Err(Error::Limit)
        ));
        assert_eq!(budget.object_usage().unwrap(), (codec, WIRE_RESERVATION));
        drop(first);
        let wire = peer.reserve_object(0, WIRE_RESERVATION).unwrap();
        let pair = budget.reserve_object(codec, 0).unwrap();
        drop(wire);
        assert_eq!(budget.object_usage().unwrap(), (codec, 0));
        drop(pair);
        assert_eq!(budget.object_usage().unwrap(), (0, 0));
    }
    #[test]
    fn configured_object_store_retains_original_inode_and_fences_replacement() {
        let parent = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let path = parent.path().join("store");
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .unwrap();
        let root = StoreRoot::open(path.clone()).unwrap();
        root.check().unwrap();
        let old = parent.path().join("original");
        std::fs::rename(&path, &old).unwrap();
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .unwrap();
        assert_eq!(root.check(), Err(Error::Policy));
        std::fs::remove_dir(&path).unwrap();
        std::fs::rename(&old, &path).unwrap();
        assert_eq!(
            root.check(),
            Err(Error::Closed),
            "namespace restoration cannot revive a fenced anchor"
        );
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o775)).unwrap();
        assert!(matches!(StoreRoot::open(path.clone()), Err(Error::Policy)));
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o775,
            "refusal must not repair an existing root"
        );
    }
}
