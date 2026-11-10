//! Explicit ordered, bounded cache mapping over the unchanged object protocol.
use super::ObjectEndpoint;
use crate::{
    Error, Id, Result,
    endpoint::{Authority, Context, OrderedEviction},
    hash,
    protocol::{Frame, Offer},
    resource_profile::SMALL_TARGET,
};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderedLane {
    Application,
    Artifact,
}
impl OrderedLane {
    fn slot(self) -> usize {
        match self {
            Self::Application => 0,
            Self::Artifact => 1,
        }
    }
}
/// Hash derivation only; it grants no scope or ordering authority.
pub fn ordered_transfer_id(domain: &[u8], scope: &Id, direction: u8, ordinal: u64) -> Id {
    hash(domain, &[scope, &[direction], &ordinal.to_be_bytes()])
}
#[derive(Clone, Debug)]
pub struct OrderedBinding {
    namespace: Id,
    scope: Id,
    direction: u8,
    domains: [&'static [u8]; 2],
}
impl OrderedBinding {
    pub fn new(
        context: &Context,
        scope: Id,
        remote_direction: u8,
        application_domain: &'static [u8],
        artifact_domain: &'static [u8],
    ) -> Result<Self> {
        if scope == [0; 32]
            || remote_direction > 1
            || application_domain == artifact_domain
            || [application_domain, artifact_domain]
                .iter()
                .any(|domain| domain.is_empty() || domain.len() > 64)
        {
            return Err(Error::Malformed);
        }
        Ok(Self {
            namespace: context.namespace()?,
            scope,
            direction: remote_direction,
            domains: [application_domain, artifact_domain],
        })
    }
    pub fn transfer_id(&self, lane: OrderedLane, ordinal: u64) -> Id {
        ordered_transfer_id(
            self.domains[lane.slot()],
            &self.scope,
            self.direction,
            ordinal,
        )
    }
}
#[derive(Clone)]
struct Current {
    ordinal: u64,
    offer: Offer,
    complete: bool,
}
#[derive(Clone, Copy)]
struct Record {
    len: u64,
    disk: u64,
    age: u64,
    pinned: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrderedSnapshot {
    pub attempts: usize,
    pub publications: usize,
    pub cache_bytes: u64,
    pub pinned: usize,
    pub evictions: u64,
}
/// One admitted connection, two immutable directional lanes. No mutable bypass.
pub struct OrderedObjectEndpoint<A: Authority> {
    inner: ObjectEndpoint<A>,
    binding: OrderedBinding,
    lanes: [Option<Current>; 2],
    cache: BTreeMap<Id, Record>,
    age: u64,
    evictions: u64,
    fenced: bool,
}
impl<A: Authority> OrderedObjectEndpoint<A> {
    pub fn new(
        inner: ObjectEndpoint<A>,
        binding: OrderedBinding,
        context: &Context,
    ) -> Result<Self> {
        inner.inner.authorize(context)?;
        if context.namespace()? != binding.namespace {
            return Err(Error::Unauthorized);
        }
        if inner.profile.capabilities().profile != 2 {
            return Err(Error::Unsupported);
        }
        inner.inner.ordered_bounds()?;
        if inner.inner.ordered_attempt_count(context)? != 0 {
            return Err(Error::Sequence);
        }
        let records = inner.inner.ordered_cache_scan(context)?;
        let mut endpoint = Self {
            inner,
            binding,
            lanes: [None, None],
            cache: BTreeMap::new(),
            age: 0,
            evictions: 0,
            fenced: false,
        };
        for (id, len, disk) in records {
            endpoint.record(id, len, disk, false)?;
        }
        endpoint.check_cache_bound()?;
        Ok(endpoint)
    }
    fn check(&self, c: &Context) -> Result<()> {
        self.inner.inner.authorize(c)?;
        if c.namespace()? != self.binding.namespace {
            return Err(Error::Unauthorized);
        }
        if self.fenced {
            return Err(Error::Storage);
        }
        Ok(())
    }
    fn tick(&mut self) -> Result<u64> {
        self.age = self.age.checked_add(1).ok_or(Error::Limit)?;
        Ok(self.age)
    }
    fn bytes(&self) -> Result<u64> {
        self.cache.values().try_fold(0u64, |total, item| {
            total.checked_add(item.disk).ok_or(Error::Limit)
        })
    }
    fn check_cache_bound(&self) -> Result<()> {
        let (count, namespace, all) = self.inner.inner.ordered_bounds()?;
        if self.cache.len() > count || self.bytes()? > namespace.min(all) {
            return Err(Error::Limit);
        }
        Ok(())
    }
    fn record(&mut self, id: Id, len: u64, disk: u64, pinned: bool) -> Result<()> {
        let age = self.tick()?;
        let pinned = pinned || self.cache.get(&id).is_some_and(|entry| entry.pinned);
        self.cache.insert(
            id,
            Record {
                len,
                disk,
                age,
                pinned,
            },
        );
        self.check_cache_bound()
    }
    fn verify_cached(&mut self, c: &Context, id: Id) -> Result<()> {
        let known = *self.cache.get(&id).ok_or(Error::Missing)?;
        match self.inner.inner.ordered_cache_meta(c, id) {
            Ok((len, disk)) if (len, disk) == (known.len, known.disk) => Ok(()),
            _ => {
                self.fenced = true;
                Err(Error::Storage)
            }
        }
    }
    fn touch(&mut self, id: Id) -> Result<()> {
        let age = self.tick()?;
        self.cache.get_mut(&id).ok_or(Error::Storage)?.age = age;
        Ok(())
    }
    fn plan_space(&mut self, c: &Context, offer: &Offer) -> Result<Vec<OrderedEviction>> {
        if let Some(known) = self.cache.get(&offer.object) {
            if known.len != offer.len {
                return Err(Error::Integrity);
            }
            self.verify_cached(c, offer.object)?;
            return Ok(Vec::new());
        }
        let mut protected = self.inner.inner.ordered_active_objects(c)?;
        for current in self.lanes.iter().flatten() {
            protected.insert(current.offer.object);
        }
        if let Some(base) = offer.base {
            self.verify_cached(c, base)?;
            protected.insert(base);
        }
        let pending = offer.len.checked_add(81).ok_or(Error::Limit)?;
        let (count, namespace, all) = self.inner.inner.ordered_bounds()?;
        let capacity = namespace.min(all);
        let mut bytes = self.bytes()?.checked_add(pending).ok_or(Error::Limit)?;
        let mut publications = self.cache.len().checked_add(1).ok_or(Error::Limit)?;
        let mut victims = Vec::new();
        if publications > count || bytes > capacity {
            let mut candidates: Vec<_> = self
                .cache
                .iter()
                .filter(|(id, record)| !record.pinned && !protected.contains(*id))
                .collect();
            candidates.sort_by_key(|(id, record)| (record.age, **id));
            for (&id, record) in candidates {
                victims.push(OrderedEviction {
                    id,
                    len: record.len,
                    disk: record.disk,
                });
                publications -= 1;
                bytes -= record.disk;
                if publications <= count && bytes <= capacity {
                    return Ok(victims);
                }
            }
            return Err(Error::Limit);
        }
        Ok(victims)
    }
    pub fn handle(
        &mut self,
        lane: OrderedLane,
        ordinal: u64,
        c: &Context,
        frame: &Frame,
        now: u64,
    ) -> Result<Frame> {
        self.check(c)?;
        if frame.transfer != self.binding.transfer_id(lane, ordinal) {
            return Err(Error::Sequence);
        }
        self.inner.profile.check_request(frame)?;
        let current = self.lanes[lane.slot()].clone();
        let fresh = match &current {
            None if ordinal == 0 => true,
            Some(old) if ordinal == old.ordinal => false,
            Some(old) if old.complete && old.ordinal.checked_add(1) == Some(ordinal) => true,
            _ => return Err(Error::Sequence),
        };
        let start = matches!(frame.kind, 1 | 4);
        if fresh && (!start || frame.seq != 0) {
            return Err(Error::Sequence);
        }
        let mut evictions = Vec::new();
        let offer = if start {
            let body = if frame.kind == 4 {
                frame.body.get(32..).ok_or(Error::Malformed)?
            } else {
                &frame.body
            };
            let offer = Offer::decode(body)?;
            if lane == OrderedLane::Application && offer.len > SMALL_TARGET as u64 {
                return Err(Error::Limit);
            }
            if !fresh && current.as_ref().is_some_and(|old| old.offer != offer) {
                return Err(Error::Sequence);
            }
            if lane == OrderedLane::Artifact
                && !self
                    .cache
                    .get(&offer.object)
                    .is_some_and(|entry| entry.pinned)
                && self.cache.values().filter(|entry| entry.pinned).count() >= 2
            {
                return Err(Error::Limit);
            }
            if fresh {
                // Probe misses do not admit state or reserve a future cache slot.
                if frame.kind == 1 {
                    evictions = self.plan_space(c, &offer)?;
                } else if self.cache.contains_key(&offer.object) {
                    self.verify_cached(c, offer.object)?;
                }
            }
            Some(offer)
        } else {
            None
        };
        let eviction_count = self
            .evictions
            .checked_add(evictions.len() as u64)
            .ok_or(Error::Limit)?;
        let reply = match self
            .inner
            .inner
            .handle_with_evictions(c, frame, now, &evictions)
        {
            Ok(reply) => reply,
            Err(error) => {
                if error == Error::Storage {
                    self.fenced = true;
                }
                return Err(error);
            }
        };
        if fresh && !matches!(reply.kind, 101 | 103) {
            self.fenced = true;
            return Err(Error::Integrity);
        }
        if !evictions.is_empty() {
            if reply.kind != 101 {
                self.fenced = true;
                return Err(Error::Integrity);
            }
            for victim in evictions {
                self.cache.remove(&victim.id);
            }
            self.evictions = eviction_count;
        }
        let admitted = offer
            .or_else(|| current.as_ref().map(|old| old.offer.clone()))
            .ok_or(Error::Sequence)?;
        let complete =
            reply.kind == 103 || current.as_ref().is_some_and(|old| !fresh && old.complete);
        if fresh && reply.kind == 101 {
            if let Some(base) = admitted.base {
                self.touch(base)?;
            }
        }
        if reply.kind == 103 {
            if reply.body != admitted.object {
                self.fenced = true;
                return Err(Error::Integrity);
            }
            let (len, disk) = match self.inner.inner.ordered_cache_meta(c, admitted.object) {
                Ok(meta) => meta,
                Err(error) => {
                    self.fenced = true;
                    return Err(error);
                }
            };
            if len != admitted.len {
                self.fenced = true;
                return Err(Error::Integrity);
            }
            if let Err(error) =
                self.record(admitted.object, len, disk, lane == OrderedLane::Artifact)
            {
                self.fenced = true;
                return Err(error);
            }
        }
        if fresh {
            if let Some(old) = current {
                if let Err(error) = self
                    .inner
                    .inner
                    .ordered_retire(c, self.binding.transfer_id(lane, old.ordinal))
                {
                    self.fenced = true;
                    return Err(error);
                }
            }
            self.lanes[lane.slot()] = Some(Current {
                ordinal,
                offer: admitted,
                complete,
            });
        } else if let Some(current) = self.lanes[lane.slot()].as_mut() {
            current.complete = complete;
        }
        Ok(reply)
    }
    pub fn read_object(&mut self, c: &Context, id: Id) -> Result<Vec<u8>> {
        self.check(c)?;
        let bytes = match self.inner.read_object(c, id) {
            Ok(bytes) => bytes,
            Err(error) => {
                if matches!(error, Error::Storage | Error::Integrity) {
                    self.fenced = true;
                }
                return Err(error);
            }
        };
        let age = self.tick()?;
        if let Some(entry) = self.cache.get_mut(&id) {
            entry.age = age;
        } else {
            self.fenced = true;
            return Err(Error::Storage);
        }
        Ok(bytes)
    }
    pub fn snapshot(&self, c: &Context) -> Result<OrderedSnapshot> {
        self.check(c)?;
        Ok(OrderedSnapshot {
            attempts: self.inner.inner.ordered_attempt_count(c)?,
            publications: self.cache.len(),
            cache_bytes: self.bytes()?,
            pinned: self.cache.values().filter(|item| item.pinned).count(),
            evictions: self.evictions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_ordinal_fence_cannot_wrap_to_old_zero_identity() {
        let root =
            std::env::temp_dir().join(format!("ordered-fence-overflow-{}", std::process::id()));
        let context = crate::wan::context([186; 32]);
        let caps = crate::v1::Capabilities::small_v1();
        let inner = ObjectEndpoint::open_with_resource_profile(
            &root,
            crate::resource_profile::ResourceProfile::SmallV1.receiver_limits(),
            crate::wan::BenchAuthority,
            caps.negotiate(&caps).unwrap(),
            crate::resource_profile::ResourceProfile::SmallV1,
        )
        .unwrap();
        let binding = OrderedBinding::new(
            &context,
            context.namespace().unwrap(),
            0,
            b"application\0",
            b"artifact\0",
        )
        .unwrap();
        let mut endpoint = OrderedObjectEndpoint::new(inner, binding.clone(), &context).unwrap();
        crate::client::transfer_batched(
            b"exact",
            crate::protocol::Mode::Raw,
            None,
            binding.transfer_id(OrderedLane::Application, 0),
            false,
            |frames| {
                frames
                    .iter()
                    .map(|frame| endpoint.handle(OrderedLane::Application, 0, &context, frame, 0))
                    .collect()
            },
        )
        .unwrap();
        // Unit-only edge setup, not a claim to have executed2^64 transfers.
        endpoint.lanes[0].as_mut().unwrap().ordinal = u64::MAX;
        let frame = Frame {
            kind: 1,
            transfer: binding.transfer_id(OrderedLane::Application, 0),
            seq: 0,
            body: Offer::cold(b"exact", crate::protocol::Mode::Raw)
                .encode()
                .unwrap(),
        };
        assert_eq!(
            endpoint.handle(OrderedLane::Application, 0, &context, &frame, 0),
            Err(Error::Sequence)
        );
        assert_eq!(endpoint.snapshot(&context).unwrap().attempts, 1);
        drop(endpoint);
        std::fs::remove_dir_all(root).unwrap();
    }
}
