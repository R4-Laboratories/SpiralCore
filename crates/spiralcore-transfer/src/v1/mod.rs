//! Transport-neutral object-v1 contract. Capabilities require authenticated
//! carrier binding or explicit trusted pinning; they grant no authority.
use crate::{
    endpoint::{
        Authority, CodecCacheStats, ColdGraphPolicy, Context, Endpoint, GraphCacheStats, Limits,
    },
    protocol::{Frame, Offer},
    Error, Id, Result, MAX_FRAME, MAX_OBJECT,
};
use std::path::Path;
mod sender;
pub use sender::{ObjectMode, ObjectReceipt, ObjectSelection, ObjectSender};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub major: u16,
    pub minor: u16,
    pub profile: u16,
    pub max_frame_bytes: u32,
    pub max_object_bytes: u64,
    pub codec_mask: u16,
}
impl Capabilities {
    pub fn small_v1() -> Self {
        Self {
            profile: 2,
            max_object_bytes: 64 * 1024,
            codec_mask: crate::resource_profile::SMALL_CODEC_MASK,
            ..Self::full()
        }
    }
    pub fn full() -> Self {
        Self {
            major: 1,
            minor: 0,
            profile: 1,
            max_frame_bytes: MAX_FRAME as u32,
            max_object_bytes: MAX_OBJECT,
            codec_mask: 1023,
        }
    }
    fn validate(&self) -> Result<()> {
        if self.major != 1
            || self.minor != 0
            || !matches!(self.profile, 1 | 2)
            || self.codec_mask & !1023 != 0
            || self.codec_mask & 1 == 0
        {
            return Err(Error::Unsupported);
        }
        if self.profile == 2
            && (self.max_object_bytes > 64 * 1024
                || self.codec_mask != crate::resource_profile::SMALL_CODEC_MASK)
        {
            return Err(Error::Unsupported);
        }
        if self.max_frame_bytes != MAX_FRAME as u32
            || self.max_object_bytes == 0
            || self.max_object_bytes > MAX_OBJECT
        {
            return Err(Error::Limit);
        }
        Ok(())
    }
    pub fn negotiate(&self, peer: &Self) -> Result<NegotiatedProfile> {
        self.validate()?;
        peer.validate()?;
        if self.profile != peer.profile {
            return Err(Error::Unsupported);
        }
        Ok(NegotiatedProfile {
            capabilities: Self {
                max_object_bytes: self.max_object_bytes.min(peer.max_object_bytes),
                codec_mask: self.codec_mask & peer.codec_mask,
                ..*self
            },
        })
    }
}
/// Validated immutable agreement. Carrier authentication remains external.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NegotiatedProfile {
    capabilities: Capabilities,
}
impl NegotiatedProfile {
    pub fn capabilities(&self) -> Capabilities {
        self.capabilities
    }
    pub fn max_object_bytes(&self) -> u64 {
        self.capabilities.max_object_bytes
    }
    pub fn max_frame_bytes(&self) -> u32 {
        self.capabilities.max_frame_bytes
    }
    pub fn codec_mask(&self) -> u16 {
        self.capabilities.codec_mask
    }
    pub fn supports(&self, mode: crate::protocol::Mode) -> bool {
        self.capabilities.codec_mask & (1 << mode as u8) != 0
    }
    pub(crate) fn check_len(&self, len: usize) -> Result<()> {
        if len as u64 > self.max_object_bytes() {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
    fn check_frame(&self, frame: &Frame) -> Result<()> {
        if frame.body.len().checked_add(56).ok_or(Error::Limit)? > self.max_frame_bytes() as usize {
            return Err(Error::Limit);
        }
        if !matches!(frame.kind,1..=6|101..=105|127) {
            return Err(Error::Unsupported);
        }
        Ok(())
    }
    fn check_request(&self, frame: &Frame) -> Result<()> {
        self.check_frame(frame)?;
        if !matches!(frame.kind, 1..=6) {
            return Err(Error::Unsupported);
        }
        let body = match frame.kind {
            1 => Some(frame.body.as_slice()),
            4 => Some(frame.body.get(32..).ok_or(Error::Malformed)?),
            _ => None,
        };
        if let Some(body) = body {
            let offer = Offer::decode(body)?;
            if !self.supports(offer.mode) {
                return Err(Error::Unsupported);
            }
            if offer.len > self.max_object_bytes() {
                return Err(Error::Limit);
            }
        }
        Ok(())
    }
    pub(crate) fn exchange<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
        &self,
        frames: &[Frame],
        exchange: &mut F,
    ) -> Result<Vec<Frame>> {
        for frame in frames {
            self.check_request(frame)?;
        }
        let replies = exchange(frames)?;
        for frame in &replies {
            self.check_frame(frame)?;
            if !matches!(frame.kind, 101..=105 | 127) {
                return Err(Error::Unsupported);
            }
        }
        Ok(replies)
    }
}
pub trait FrameEndpoint {
    fn handle(&mut self, context: &Context, frame: &Frame, now: u64) -> Result<Frame>;
}
impl<A: Authority> FrameEndpoint for Endpoint<A> {
    fn handle(&mut self, c: &Context, f: &Frame, now: u64) -> Result<Frame> {
        Endpoint::handle(self, c, f, now)
    }
}
/// A fresh core receiver with a fixed admission profile and no mutable bypass.
pub struct ObjectEndpoint<A: Authority> {
    inner: Endpoint<A>,
    profile: NegotiatedProfile,
}
impl<A: Authority> ObjectEndpoint<A> {
    pub fn open(
        root: &Path,
        limits: Limits,
        authority: A,
        profile: NegotiatedProfile,
    ) -> Result<Self> {
        Self::open_with_policy(
            root,
            limits,
            authority,
            profile,
            ColdGraphPolicy::EagerBounded,
        )
    }
    pub fn open_with_policy(
        root: &Path,
        limits: Limits,
        authority: A,
        profile: NegotiatedProfile,
        policy: ColdGraphPolicy,
    ) -> Result<Self> {
        crate::resource_profile::ResourceProfile::General.check_negotiated(profile)?;
        Ok(Self {
            inner: Endpoint::open_with_policy(root, limits, authority, policy)?,
            profile,
        })
    }
    pub fn open_with_resource_profile(
        root: &Path,
        limits: Limits,
        authority: A,
        profile: NegotiatedProfile,
        resources: crate::resource_profile::ResourceProfile,
    ) -> Result<Self> {
        resources.check_negotiated(profile)?;
        Ok(Self {
            inner: Endpoint::open_with_resource_profile(root, limits, authority, resources)?,
            profile,
        })
    }
    pub fn profile(&self) -> NegotiatedProfile {
        self.profile
    }
    pub fn handle(&mut self, c: &Context, f: &Frame, now: u64) -> Result<Frame> {
        self.inner.authorize(c)?;
        c.namespace()?;
        self.profile.check_request(f)?;
        self.inner.handle(c, f, now)
    }
    pub fn read_object(&mut self, c: &Context, id: Id) -> Result<Vec<u8>> {
        self.inner.read_object(c, id)
    }
    pub fn set_codec_cache_enabled(&mut self, enabled: bool) -> Result<()> {
        self.inner.set_codec_cache_enabled(enabled)
    }
    pub fn codec_cache_stats(&self) -> CodecCacheStats {
        self.inner.codec_cache_stats()
    }
    pub fn graph_cache_stats(&self) -> GraphCacheStats {
        self.inner.graph_cache_stats()
    }
}
impl<A: Authority> FrameEndpoint for ObjectEndpoint<A> {
    fn handle(&mut self, c: &Context, f: &Frame, now: u64) -> Result<Frame> {
        ObjectEndpoint::handle(self, c, f, now)
    }
}
