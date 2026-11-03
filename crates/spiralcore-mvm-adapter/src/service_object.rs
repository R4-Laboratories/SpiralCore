//! Scoped bridge from an admitted canonical Call to the shared object-v1 core.
use crate::{
    MailboxAuthority, ObjectEndpoint, Policy, Result, content_type,
    direction::verified_object_context,
    service::{AdmittedMessage, MailboxBinding, ServiceError, ServiceTime},
};
use mvm_contract::protocol::mailbox::MailboxMessageKind;
use spiralcore_transfer::{
    Id,
    endpoint::{ColdGraphPolicy, Context, Limits},
    protocol::Frame,
};
use std::path::Path;

/// The receiver never exposes a mutable inner endpoint or a reusable authority.
/// Each operation requires a current private admitted-message proof.
pub struct ObjectMailbox {
    endpoint: ObjectEndpoint,
    policy: Policy,
    authority: MailboxAuthority,
}
impl ObjectMailbox {
    pub fn set_codec_cache_enabled(
        &mut self,
        enabled: bool,
    ) -> std::result::Result<(), ServiceError> {
        self.endpoint
            .set_codec_cache_enabled(enabled)
            .map_err(map_transfer)
    }

    /// Namespace bookkeeping from a current admitted proof; this returns no
    /// reusable authority and does not permit access to the inner receiver.
    pub fn context(
        &self,
        binding: &MailboxBinding,
        proof: &AdmittedMessage,
        now: ServiceTime,
    ) -> std::result::Result<Context, ServiceError> {
        proof.with_verified(binding, now, |current, header, _| {
            if header.kind != MailboxMessageKind::Call
                || header.content_type != content_type().map_err(|_| ServiceError::State)?
            {
                return Err(ServiceError::Unauthorized);
            }
            verified_object_context(current, binding.capability_grant())
                .map_err(|_| ServiceError::State)
        })?
    }
    pub fn open(
        root: &Path,
        limits: Limits,
        policy: Policy,
        cache: ColdGraphPolicy,
    ) -> Result<Self> {
        let authority = MailboxAuthority::new(policy.clone());
        let endpoint = crate::open_object_endpoint(root, limits, authority.clone(), cache)?;
        Ok(Self {
            endpoint,
            policy,
            authority,
        })
    }

    pub fn handle(
        &mut self,
        binding: &MailboxBinding,
        proof: &AdmittedMessage,
        now: ServiceTime,
    ) -> std::result::Result<Frame, ServiceError> {
        self.with_scope(binding, proof, now, |endpoint, context, frame| {
            Ok(
                match endpoint.handle(context, frame, now.monotonic_ms / 1000) {
                    Ok(reply) => reply,
                    Err(error) => frame.reply(127, (error as u16).to_le_bytes().to_vec()),
                },
            )
        })
    }

    pub fn read_object(
        &mut self,
        binding: &MailboxBinding,
        proof: &AdmittedMessage,
        id: Id,
        now: ServiceTime,
    ) -> std::result::Result<Vec<u8>, ServiceError> {
        self.with_scope(binding, proof, now, |endpoint, context, _| {
            endpoint.read_object(context, id)
        })
    }

    fn with_scope<T>(
        &mut self,
        binding: &MailboxBinding,
        proof: &AdmittedMessage,
        now: ServiceTime,
        action: impl FnOnce(&mut ObjectEndpoint, &Context, &Frame) -> spiralcore_transfer::Result<T>,
    ) -> std::result::Result<T, ServiceError> {
        proof.with_verified(binding, now, |current, header, payload| {
            let object_type = content_type().map_err(|_| ServiceError::State)?;
            if header.kind != MailboxMessageKind::Call || header.content_type != object_type {
                return Err(ServiceError::Unauthorized);
            }
            let frame = Frame::decode(payload).map_err(map_transfer)?;
            if !matches!(frame.kind, 1..=6) {
                return Err(ServiceError::Malformed);
            }
            let context = verified_object_context(current, binding.capability_grant())
                .map_err(|_| ServiceError::State)?;
            let mut object_policy = current.clone();
            object_policy.accepted_content_types = vec![object_type];
            self.policy
                .replace(object_policy)
                .map_err(|_| ServiceError::Unauthorized)?;
            let generation = self.policy.versioned().map_err(|_| ServiceError::State)?.1;
            self.authority
                .install(context.clone(), generation)
                .map_err(|_| ServiceError::Unauthorized)?;
            let result = action(&mut self.endpoint, &context, &frame).map_err(map_transfer);
            self.authority.clear().map_err(|_| ServiceError::State)?;
            result
        })?
    }
}
fn map_transfer(error: spiralcore_transfer::Error) -> ServiceError {
    use spiralcore_transfer::Error;
    match error {
        Error::Unauthorized => ServiceError::Unauthorized,
        Error::Limit => ServiceError::Limit,
        Error::Expired => ServiceError::Expired,
        Error::Storage => ServiceError::State,
        _ => ServiceError::Malformed,
    }
}
