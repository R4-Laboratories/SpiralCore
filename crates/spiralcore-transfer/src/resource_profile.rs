//! Local admission bounds for the explicitly agreed SmallV1 codec profile.
//! Declared bytes are not RSS. An embedding reserves one aggregate envelope
//! before constructing both endpoints and keeps it through all retained state.
use crate::{Error, Result, endpoint::Limits, protocol::Mode};
use zstd::zstd_safe::{CCtx, CParameter, Strategy};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResourceProfile {
    #[default]
    General,
    SmallV1,
}
const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;
pub const SMALL_TARGET: usize = 16 * 1024;
pub const SMALL_ARTIFACT: usize = 64 * 1024;
pub(crate) const SMALL_CODEC_MASK: u16 = (1 << Mode::Raw as u8)
    | (1 << Mode::SparseV1 as u8)
    | (1 << Mode::ZstdPrefixV1 as u8)
    | (1 << Mode::RuntimeV1 as u8)
    | (1 << Mode::SchemaV1 as u8)
    | (1 << Mode::SchemaZstdV1 as u8)
    | (1 << Mode::ZstdWholeV1 as u8);

impl ResourceProfile {
    pub fn max_target_len(self) -> usize {
        match self {
            Self::General => crate::client::MAX_BATCHED_OBJECT,
            Self::SmallV1 => SMALL_TARGET,
        }
    }
    pub fn max_artifact_len(self) -> usize {
        SMALL_ARTIFACT
    }
    pub fn max_prediction_len(self) -> usize {
        match self {
            Self::General => crate::shared_runtime::MAX_OUTPUT,
            Self::SmallV1 => SMALL_ARTIFACT,
        }
    }
    pub(crate) fn check_supported(self) -> Result<()> {
        if self == Self::SmallV1
            && (usize::BITS != 64 || !crate::zstd_whole::first_block_probe_available())
        {
            return Err(Error::Unsupported);
        }
        Ok(())
    }
    pub(crate) fn check_target(self, len: usize, capacity: usize) -> Result<()> {
        self.check_supported()?;
        if len > self.max_target_len() || capacity > self.max_target_len() {
            return Err(Error::Limit);
        }
        Ok(())
    }
    pub(crate) fn check_artifact(self, len: usize) -> Result<()> {
        self.check_supported()?;
        if len > self.max_artifact_len() {
            return Err(Error::Limit);
        }
        Ok(())
    }
    pub(crate) fn check_offer(self, offer: &crate::protocol::Offer) -> Result<()> {
        if self == Self::General {
            return Ok(());
        }
        self.check_supported()?;
        if SMALL_CODEC_MASK & (1 << offer.mode as u8) == 0 {
            return Err(Error::Unsupported);
        }
        let limit = if offer.mode == Mode::Raw {
            self.max_artifact_len()
        } else {
            self.max_target_len()
        };
        if offer.len > limit as u64 {
            return Err(Error::Limit);
        }
        Ok(())
    }
    /// Pinned single-shot fast strategy: hash table only; minMatch4 implies
    /// <=n/4 sequences, each8-byte SeqDef plus3 code bytes. The4*n term
    /// bounds those plus literals. Fixed256KiB covers CCtx, two entropy block
    /// states, <=9KiB temporary workspace and alignment/ABI bookkeeping.
    /// No LDM, chain/optimal/row tables, stream buffers, CDict or workers.
    pub(crate) fn encoder_charge(self, source_bound: usize) -> Result<u64> {
        if self == Self::General {
            return Ok(crate::zstd_dictionary::ENCODER_ALLOWANCE);
        }
        self.check_supported()?;
        if source_bound > crate::schema::encoded_cap(SMALL_TARGET)? {
            return Err(Error::Limit);
        }
        (256 * KIB + 4 * (1 << 14) + 4 * KIB)
            .checked_add(
                u64::try_from(source_bound)
                    .map_err(|_| Error::Limit)?
                    .checked_mul(4)
                    .ok_or(Error::Limit)?,
            )
            .ok_or(Error::Limit)
    }
    pub(crate) fn decoder_charge(self) -> u64 {
        match self {
            Self::General => crate::zstd_prefix::DECODER_ALLOWANCE,
            Self::SmallV1 => 512 * KIB,
        }
    }
    pub(crate) fn transient(self, runtime: bool) -> u64 {
        match self {
            Self::General if runtime => 384 * MIB,
            Self::General => 128 * MIB,
            Self::SmallV1 => 2 * MIB,
        }
    }
    pub(crate) fn sender_charge(self) -> Result<u64> {
        if self == Self::General {
            return Ok(crate::session::MAX_SESSION_CHARGE);
        }
        let contexts = self
            .encoder_charge(SMALL_TARGET)?
            .checked_add(self.encoder_charge(crate::schema::encoded_cap(SMALL_TARGET)?)?)
            .ok_or(Error::Limit)?;
        contexts
            .checked_add(crate::schema::PREPARED_CHARGE)
            .and_then(|n| n.checked_add(SMALL_TARGET as u64 + 4 * SMALL_ARTIFACT as u64))
            .and_then(|n| n.checked_add(2 * MIB))
            .ok_or(Error::Limit)
    }
    pub(crate) fn receiver_charge(self) -> Result<u64> {
        if self == Self::General {
            return Ok(Limits::default().resident_bytes);
        }
        let encoded = crate::schema_zstd::encoded_cap(SMALL_TARGET)? as u64;
        (2 * MIB)
            .checked_add(encoded.checked_mul(4).ok_or(Error::Limit)?)
            .and_then(|n| n.checked_add(6 * SMALL_ARTIFACT as u64))
            .and_then(|n| n.checked_add(self.decoder_charge()))
            .and_then(|n| n.checked_add(crate::schema::PREPARED_CHARGE))
            .and_then(|n| n.checked_add(328 * KIB))
            .ok_or(Error::Limit)
    }
    pub fn operation_charge(self) -> Result<u64> {
        self.check_supported()?;
        let charge = self
            .sender_charge()?
            .checked_add(self.receiver_charge()?)
            .ok_or(Error::Limit)?;
        if self == Self::SmallV1 && charge > 8 * MIB {
            return Err(Error::Limit);
        }
        Ok(charge)
    }
    pub fn receiver_limits(self) -> Limits {
        if self == Self::General {
            return Limits::default();
        }
        Limits {
            max_nodes: 1,
            max_active: 1,
            resident_bytes: self.receiver_charge().unwrap_or(0),
            namespace_bytes: 8 * MIB,
            cache_bytes: 8 * MIB,
            graph_cache_bytes: 0,
            graph_cache_entries: 0,
            max_states: 32,
            max_publications: 64,
            ..Limits::default()
        }
    }
    pub(crate) fn check_limits(self, limits: &Limits) -> Result<()> {
        self.check_supported()?;
        if self == Self::SmallV1
            && (limits.max_active > 1
                || limits.max_states > 32
                || limits.max_publications > 64
                || limits.graph_cache_bytes != 0
                || limits.graph_cache_entries != 0
                || limits.resident_bytes > self.receiver_charge()?
                || limits.cache_bytes > 8 * MIB
                || limits.namespace_bytes > 8 * MIB)
        {
            return Err(Error::Limit);
        }
        Ok(())
    }
    pub(crate) fn check_negotiated(self, profile: crate::v1::NegotiatedProfile) -> Result<()> {
        self.check_supported()?;
        let expected = if self == Self::SmallV1 { 2 } else { 1 };
        if profile.capabilities().profile != expected {
            return Err(Error::Unsupported);
        }
        Ok(())
    }
    pub(crate) fn encoder(self, source_bound: usize) -> Result<CCtx<'static>> {
        if self == Self::General {
            return crate::zstd_dictionary::new_encoder();
        }
        let charge = self.encoder_charge(source_bound)?;
        let mut encoder = CCtx::try_create().ok_or(Error::Limit)?;
        for p in [
            CParameter::CompressionLevel(3),
            CParameter::WindowLog(17),
            CParameter::HashLog(14),
            CParameter::ChainLog(14),
            CParameter::MinMatch(4),
            CParameter::SearchLog(1),
            CParameter::Strategy(Strategy::ZSTD_fast),
            CParameter::EnableLongDistanceMatching(false),
            CParameter::NbWorkers(0),
            CParameter::ContentSizeFlag(true),
            CParameter::ChecksumFlag(true),
            CParameter::DictIdFlag(false),
        ] {
            encoder.set_parameter(p).map_err(|_| Error::Unsupported)?;
        }
        if encoder.sizeof() as u64 > charge {
            return Err(Error::Limit);
        }
        Ok(encoder)
    }
}
