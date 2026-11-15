//! Fixed SmallV1 filesystem metadata capacities; no protocol or authority.
use crate::{endpoint::OrderedEviction, Error, Id, Result};

pub(crate) const MAX_PUBLICATIONS: usize = 64;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) const EXISTING_STACK_BYTES: u64 = 2 << 20;
pub(crate) type ScanRecord = (Id, u64, u64);

#[derive(Clone, Copy)]
pub(crate) struct CacheRecord {
    pub(crate) len: u64,
    pub(crate) disk: u64,
    pub(crate) age: u64,
    pub(crate) pinned: bool,
}
struct CacheEntry {
    id: Id,
    record: CacheRecord,
}

fn fixed_vec<T>(count: usize) -> Result<Vec<T>> {
    if count > MAX_PUBLICATIONS {
        return Err(Error::Limit);
    }
    let mut entries = Vec::new();
    entries.try_reserve_exact(count).map_err(|_| Error::Limit)?;
    if entries.capacity() != count {
        return Err(Error::Limit);
    }
    Ok(entries)
}

pub(crate) struct CacheIndex {
    // Fields drop in declaration order: storage must disappear before its
    // original lease can become exclusive and authorize final retirement.
    entries: Vec<CacheEntry>,
    limit: usize,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    custody: Option<crate::endpoint::OwnedStoreLease>,
}
impl CacheIndex {
    pub(crate) fn candidates(&self, protected: &ProtectedIds) -> Result<Candidates<'_>> {
        let mut candidates = Candidates {
            entries: [None; MAX_PUBLICATIONS],
            len: 0,
        };
        for (id, record) in self.iter() {
            if !record.pinned && !protected.contains(id) {
                *candidates
                    .entries
                    .get_mut(candidates.len)
                    .ok_or(Error::Limit)? = Some((id, record));
                candidates.len += 1;
            }
        }
        // No auxiliary heap or recursion. The total (age, ID) key preserves
        // the previous stable-sort result, including equal ages.
        for first in 0..candidates.len {
            let mut least = first;
            for next in first + 1..candidates.len {
                let a = candidates.entries[next].ok_or(Error::Storage)?;
                let b = candidates.entries[least].ok_or(Error::Storage)?;
                if (a.1.age, *a.0) < (b.1.age, *b.0) {
                    least = next;
                }
            }
            candidates.entries.swap(first, least);
        }
        Ok(candidates)
    }
    fn new(limit: usize) -> Result<Self> {
        Ok(Self {
            entries: fixed_vec(limit)?,
            limit,
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            custody: None,
        })
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub(crate) fn bind_lease(&mut self, lease: crate::endpoint::OwnedStoreLease) -> Result<()> {
        if self.custody.is_some() {
            return Err(Error::Storage);
        }
        self.custody = Some(lease);
        Ok(())
    }
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
    pub(crate) fn get(&self, id: &Id) -> Option<&CacheRecord> {
        self.entries
            .iter()
            .find(|entry| &entry.id == id)
            .map(|entry| &entry.record)
    }
    pub(crate) fn get_mut(&mut self, id: &Id) -> Option<&mut CacheRecord> {
        self.entries
            .iter_mut()
            .find(|entry| &entry.id == id)
            .map(|entry| &mut entry.record)
    }
    pub(crate) fn contains_key(&self, id: &Id) -> bool {
        self.get(id).is_some()
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&Id, &CacheRecord)> {
        self.entries.iter().map(|entry| (&entry.id, &entry.record))
    }
    pub(crate) fn values(&self) -> impl Iterator<Item = &CacheRecord> {
        self.entries.iter().map(|entry| &entry.record)
    }
    pub(crate) fn insert(&mut self, id: Id, record: CacheRecord) -> Result<()> {
        if let Some(existing) = self.get_mut(&id) {
            *existing = record;
        } else {
            if self.entries.len() == self.limit {
                return Err(Error::Limit);
            }
            self.entries.push(CacheEntry { id, record });
        }
        Ok(())
    }
    pub(crate) fn remove(&mut self, id: &Id) -> Option<CacheRecord> {
        let index = self.entries.iter().position(|entry| &entry.id == id)?;
        Some(self.entries.remove(index).record)
    }
}

pub(crate) struct Candidates<'a> {
    entries: [Option<(&'a Id, &'a CacheRecord)>; MAX_PUBLICATIONS],
    len: usize,
}
impl<'a> Candidates<'a> {
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&'a Id, &'a CacheRecord)> + '_ {
        self.entries[..self.len].iter().filter_map(|entry| *entry)
    }
}

pub(crate) struct PreparedIndex {
    // The unconsumed scan vector also drops before cache's lease guard.
    pub(crate) scan: Vec<ScanRecord>,
    pub(crate) cache: CacheIndex,
}
impl PreparedIndex {
    pub(crate) fn new(limit: usize) -> Result<Self> {
        Ok(Self {
            cache: CacheIndex::new(limit)?,
            scan: fixed_vec(limit)?,
        })
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub(crate) fn layout_bytes(limit: usize) -> Result<(u64, u64)> {
        if limit > MAX_PUBLICATIONS {
            return Err(Error::Limit);
        }
        let index = limit
            .checked_mul(std::mem::size_of::<CacheEntry>())
            .ok_or(Error::Limit)?;
        let scan = limit
            .checked_mul(std::mem::size_of::<ScanRecord>())
            .ok_or(Error::Limit)?;
        Ok((
            u64::try_from(index).map_err(|_| Error::Limit)?,
            u64::try_from(scan).map_err(|_| Error::Limit)?,
        ))
    }
}

#[derive(Default)]
pub(crate) struct ProtectedIds {
    entries: [Id; 5],
    len: usize,
}
impl ProtectedIds {
    pub(crate) fn contains(&self, id: &Id) -> bool {
        self.entries[..self.len].contains(id)
    }
    pub(crate) fn insert(&mut self, id: Id) -> Result<()> {
        if !self.contains(&id) {
            *self.entries.get_mut(self.len).ok_or(Error::Limit)? = id;
            self.len += 1;
        }
        Ok(())
    }
}

pub(crate) struct Evictions {
    entries: [OrderedEviction; MAX_PUBLICATIONS],
    len: usize,
}
impl Default for Evictions {
    fn default() -> Self {
        Self {
            entries: [OrderedEviction {
                id: [0; 32],
                len: 0,
                disk: 0,
            }; MAX_PUBLICATIONS],
            len: 0,
        }
    }
}
impl Evictions {
    pub(crate) fn len(&self) -> usize {
        self.len
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub(crate) fn as_slice(&self) -> &[OrderedEviction] {
        &self.entries[..self.len]
    }
    pub(crate) fn push(&mut self, value: OrderedEviction) -> Result<()> {
        *self.entries.get_mut(self.len).ok_or(Error::Limit)? = value;
        self.len += 1;
        Ok(())
    }
}

/// General endpoints admit at most four active attempts plus the new offer.
#[derive(Default)]
pub(crate) struct PendingDisk {
    entries: [((Id, Id), u64); 5],
    len: usize,
}
impl PendingDisk {
    pub(crate) fn insert(&mut self, key: (Id, Id), bytes: u64) -> Result<()> {
        if let Some(entry) = self.entries[..self.len]
            .iter_mut()
            .find(|entry| entry.0 == key)
        {
            entry.1 = bytes;
        } else {
            *self.entries.get_mut(self.len).ok_or(Error::Limit)? = (key, bytes);
            self.len += 1;
        }
        Ok(())
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = &((Id, Id), u64)> {
        self.entries[..self.len].iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(age: u64, pinned: bool) -> CacheRecord {
        CacheRecord {
            len: 1,
            disk: 50,
            age,
            pinned,
        }
    }

    #[test]
    fn cache_capacity_refusal_preserves_entries_and_updates_without_growth() {
        for limit in [0, 1, 64] {
            let mut cache = CacheIndex::new(limit).unwrap();
            assert_eq!(cache.entries.capacity(), limit);
            for value in 0..limit {
                cache
                    .insert([value as u8; 32], record(value as u64, false))
                    .unwrap();
            }
            let before: Vec<_> = cache.iter().map(|(id, r)| (*id, r.age)).collect();
            assert!(matches!(
                cache.insert([255; 32], record(0, false)),
                Err(Error::Limit)
            ));
            assert_eq!(
                cache.iter().map(|(id, r)| (*id, r.age)).collect::<Vec<_>>(),
                before
            );
            if limit != 0 {
                cache.insert([0; 32], record(99, true)).unwrap();
                assert_eq!(cache.get(&[0; 32]).unwrap().age, 99);
                assert!(cache.remove(&[0; 32]).is_some());
                cache.insert([255; 32], record(1, false)).unwrap();
            }
            assert_eq!(cache.entries.capacity(), limit);
        }
        assert!(matches!(PreparedIndex::new(65), Err(Error::Limit)));
    }

    #[test]
    fn candidates_preserve_legacy_total_order_and_all_protections() {
        let mut cache = CacheIndex::new(64).unwrap();
        for ordinal in 0..64 {
            let value = (ordinal * 17) % 64;
            cache
                .insert([value; 32], record(u64::from(value % 7), value % 9 == 0))
                .unwrap();
        }
        let mut protected = ProtectedIds::default();
        for value in [3, 5, 17, 20, 61] {
            protected.insert([value; 32]).unwrap();
        }
        let mut legacy: Vec<_> = cache
            .iter()
            .filter(|(id, r)| !r.pinned && !protected.contains(id))
            .map(|(id, r)| (*id, r.age))
            .collect();
        legacy.sort_by_key(|(id, age)| (*age, *id));
        let actual: Vec<_> = cache
            .candidates(&protected)
            .unwrap()
            .iter()
            .map(|(id, r)| (*id, r.age))
            .collect();
        assert_eq!(actual, legacy);
    }

    #[test]
    fn protected_and_pending_capacities_refuse_without_losing_originals() {
        let mut protected = ProtectedIds::default();
        let mut pending = PendingDisk::default();
        for value in 0..5 {
            protected.insert([value; 32]).unwrap();
            pending
                .insert(([value; 32], [value; 32]), u64::from(value))
                .unwrap();
        }
        protected.insert([0; 32]).unwrap();
        assert_eq!(protected.len, 5);
        assert!(matches!(protected.insert([9; 32]), Err(Error::Limit)));
        assert!(matches!(
            pending.insert(([9; 32], [9; 32]), 9),
            Err(Error::Limit)
        ));
        pending.insert(([0; 32], [0; 32]), 99).unwrap();
        assert_eq!(pending.iter().count(), 5);
        assert_eq!(
            pending
                .iter()
                .find(|entry| entry.0 == ([0; 32], [0; 32]))
                .unwrap()
                .1,
            99
        );
        for value in 0..5 {
            assert!(protected.contains(&[value; 32]));
        }
    }

    #[test]
    fn victim_capacity_refuses_before_any_existing_selection_changes() {
        let mut victims = Evictions::default();
        for value in 0..64 {
            victims
                .push(OrderedEviction {
                    id: [value; 32],
                    len: 1,
                    disk: 50,
                })
                .unwrap();
        }
        assert!(matches!(
            victims.push(OrderedEviction {
                id: [255; 32],
                len: 1,
                disk: 50
            }),
            Err(Error::Limit)
        ));
        assert_eq!(victims.len(), 64);
        assert_eq!(victims.as_slice().first().unwrap().id, [0; 32]);
        assert_eq!(victims.as_slice().last().unwrap().id, [63; 32]);
    }
}
