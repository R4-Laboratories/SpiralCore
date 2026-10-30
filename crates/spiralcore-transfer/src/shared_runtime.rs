//! Bounded replay of the existing SpiralCore ternary/phase-memory/CA runtime.
//!
//! This profile preserves the Python transition equations, not its serialized
//! cards or platform floating-point bits. Canonical Rust checkpoints are data:
//! no received source, executable code, clock access, or network access occurs.
use crate::{protocol::Reader, Error, Result};

pub const MAX_CHECKPOINT: usize = 64 * 1024;
pub const MAX_OUTPUT: usize = 16 * 1024 * 1024;
pub const MAX_STEPS: u32 = 4096;
pub const MAX_WIDTH: usize = 4096;
const MAX_N: u64 = 1_000_000_000;
const MAX_ROOTS: usize = 81;
const MAX_WORK: u64 = 134_217_728;
const TAU: f64 = std::f64::consts::TAU;
const CHECKPOINT_MAGIC: &[u8; 8] = b"SCRUN001";
const RECIPE_MAGIC: &[u8; 8] = b"SCRCP001";
const TRACE_MAGIC: &[u8; 8] = b"SCTRAC01";
const CHECKPOINT_FIXED_BYTES: usize = 310;
const ROOT_BYTES: usize = 80;
// Original random.Random(184).uniform(0, 2*pi), before the initial 0.05 step.
const INITIAL_PHASE_BITS: [u64; 16] = [
    0x3ffbbd00d1899193,
    0x3ff82d47b613e279,
    0x40176a7a028b8550,
    0x401198bf1630a82c,
    0x4000c0701d686b50,
    0x40145bcb89b2afbc,
    0x4016aee1250e71a1,
    0x4017ac85205d809d,
    0x400f181aa6ada2b3,
    0x40174d259e82a794,
    0x40153dee7e8d3a66,
    0x400d6fc84589f592,
    0x3fa93bfc91c3c12c,
    0x4009a5ca209a2e6d,
    0x4000d6e7c72597e7,
    0x40020067013de538,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    FinalCheckpoint,
    Trace,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Recipe {
    pub steps: u32,
    pub payload: f64,
    pub output: OutputKind,
}

impl Recipe {
    fn validate(&self) -> Result<()> {
        if self.steps > MAX_STEPS {
            return Err(Error::Limit);
        }
        bounded_float(self.payload, 1e12)
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let mut b = RECIPE_MAGIC.to_vec();
        b.extend_from_slice(&self.steps.to_le_bytes());
        put_float(&mut b, self.payload);
        b.push(match self.output {
            OutputKind::FinalCheckpoint => 0,
            OutputKind::Trace => 1,
        });
        b.extend_from_slice(&[0; 3]);
        Ok(b)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        if r.take(8)? != RECIPE_MAGIC {
            return Err(Error::Unsupported);
        }
        let steps = r.u32()?;
        let payload = read_float(&mut r)?;
        let output = match r.u8()? {
            0 => OutputKind::FinalCheckpoint,
            1 => OutputKind::Trace,
            _ => return Err(Error::Unsupported),
        };
        if r.take(3)? != [0; 3] {
            return Err(Error::Malformed);
        }
        r.end()?;
        let recipe = Self {
            steps,
            payload,
            output,
        };
        recipe.validate()?;
        Ok(recipe)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Root {
    pub value: f64,
    pub memory_count: u64,
    pub phase_sum: [[f64; 2]; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub n: u64,
    pub width: usize,
    pub phases: [f64; 16],
    pub omegas: [f64; 16],
    pub roots: Vec<Root>,
    /// Rule 184, 184 counterfactual, 110, 110 counterfactual, 30, 30 counterfactual.
    pub rings: [Vec<u8>; 6],
    pub fixed_dt: f64,
    pub memory_enabled: bool,
    pub memory_gain: f64,
    pub k_coupling: f64,
    pub kuramoto_k: f64,
}

impl State {
    pub fn seeded(width: usize) -> Result<Self> {
        if !(1..=MAX_WIDTH).contains(&width) {
            return Err(Error::Limit);
        }
        let row: Vec<u8> = (0..width)
            .map(|i| u8::from(i % 2 == 0 && !(width / 3 <= i && i < 2 * width / 3 && i % 3 == 0)))
            .collect();
        let counterfactual: Vec<u8> = row.iter().map(|b| 1 - b).collect();
        let mut state = Self {
            n: 0,
            width,
            phases: INITIAL_PHASE_BITS.map(f64::from_bits),
            omegas: std::array::from_fn(|i| 0.8 + (0.8 * i as f64) / 15.0),
            roots: Vec::new(),
            rings: std::array::from_fn(|i| {
                if i % 2 == 0 {
                    row.clone()
                } else {
                    counterfactual.clone()
                }
            }),
            fixed_dt: 0.35,
            memory_enabled: true,
            memory_gain: 0.35,
            k_coupling: 0.15,
            kuramoto_k: 1.75,
        };
        state.kuramoto(0.05);
        Ok(state)
    }

    fn validate(&self) -> Result<()> {
        if !(1..=MAX_WIDTH).contains(&self.width) || self.n > MAX_N || self.roots.len() > MAX_ROOTS
        {
            return Err(Error::Limit);
        }
        if !self.fixed_dt.is_finite() || self.fixed_dt <= 0.0 || self.fixed_dt > 5.0 {
            return Err(Error::Malformed);
        }
        for value in [self.memory_gain, self.k_coupling, self.kuramoto_k] {
            bounded_float(value, 16.0)?;
        }
        for &phase in &self.phases {
            if !phase.is_finite() || !(0.0..TAU).contains(&phase) {
                return Err(Error::Malformed);
            }
        }
        for &omega in &self.omegas {
            bounded_float(omega, 16.0)?;
        }
        let mut count = 0u64;
        for root in &self.roots {
            bounded_float(root.value, 1e12)?;
            if root.memory_count == 0 || root.memory_count > MAX_N {
                return Err(Error::Limit);
            }
            count = count.checked_add(root.memory_count).ok_or(Error::Limit)?;
            // Summed unit phasors can accumulate tiny floating-point roundoff.
            let bound = root.memory_count as f64 * (1.0 + 1e-12);
            for component in root.phase_sum.iter().flatten() {
                bounded_float(*component, bound)?;
            }
        }
        if count > self.n {
            return Err(Error::Malformed);
        }
        for row in &self.rings {
            if row.len() != self.width || row.iter().any(|&b| b > 1) {
                return Err(Error::Malformed);
            }
        }
        Ok(())
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let mut b = Vec::with_capacity(checkpoint_size(self.width, self.roots.len()));
        b.extend_from_slice(CHECKPOINT_MAGIC);
        b.extend_from_slice(&self.n.to_le_bytes());
        b.extend_from_slice(&(self.width as u16).to_le_bytes());
        b.extend_from_slice(&[u8::from(self.memory_enabled), 0]);
        for value in [
            self.fixed_dt,
            self.memory_gain,
            self.k_coupling,
            self.kuramoto_k,
        ] {
            put_float(&mut b, value);
        }
        for value in self.phases.iter().chain(self.omegas.iter()) {
            put_float(&mut b, *value);
        }
        b.extend_from_slice(&(self.roots.len() as u16).to_le_bytes());
        for root in &self.roots {
            put_float(&mut b, root.value);
            b.extend_from_slice(&root.memory_count.to_le_bytes());
            for value in root.phase_sum.iter().flatten() {
                put_float(&mut b, *value);
            }
        }
        for row in &self.rings {
            for chunk in row.chunks(8) {
                let mut byte = 0;
                for (i, bit) in chunk.iter().enumerate() {
                    byte |= bit << (7 - i);
                }
                b.push(byte);
            }
        }
        Ok(b)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_CHECKPOINT {
            return Err(Error::Limit);
        }
        let mut r = Reader::new(bytes);
        if r.take(8)? != CHECKPOINT_MAGIC {
            return Err(Error::Unsupported);
        }
        let n = r.u64()?;
        let width = r.u16()? as usize;
        if !(1..=MAX_WIDTH).contains(&width) {
            return Err(Error::Limit);
        }
        let memory_enabled = match r.u8()? {
            0 => false,
            1 => true,
            _ => return Err(Error::Malformed),
        };
        if r.u8()? != 0 {
            return Err(Error::Malformed);
        }
        let fixed_dt = read_float(&mut r)?;
        let memory_gain = read_float(&mut r)?;
        let k_coupling = read_float(&mut r)?;
        let kuramoto_k = read_float(&mut r)?;
        let mut phases = [0.0; 16];
        let mut omegas = [0.0; 16];
        for value in phases.iter_mut().chain(omegas.iter_mut()) {
            *value = read_float(&mut r)?;
        }
        let root_count = r.u16()? as usize;
        if root_count > MAX_ROOTS {
            return Err(Error::Limit);
        }
        let mut roots = Vec::with_capacity(root_count);
        for _ in 0..root_count {
            let value = read_float(&mut r)?;
            let memory_count = r.u64()?;
            let mut phase_sum = [[0.0; 2]; 4];
            for component in phase_sum.iter_mut().flatten() {
                *component = read_float(&mut r)?;
            }
            roots.push(Root {
                value,
                memory_count,
                phase_sum,
            });
        }
        let mut rings: [Vec<u8>; 6] = std::array::from_fn(|_| Vec::with_capacity(width));
        for row in &mut rings {
            let encoded = r.take(width.div_ceil(8))?;
            let remainder = width % 8;
            if remainder != 0 && encoded[encoded.len() - 1] & ((1 << (8 - remainder)) - 1) != 0 {
                return Err(Error::Malformed);
            }
            row.extend((0..width).map(|i| (encoded[i / 8] >> (7 - i % 8)) & 1));
        }
        r.end()?;
        let state = Self {
            n,
            width,
            phases,
            omegas,
            roots,
            rings,
            fixed_dt,
            memory_enabled,
            memory_gain,
            k_coupling,
            kuramoto_k,
        };
        state.validate()?;
        Ok(state)
    }

    /// Execute one existing-runtime transition. No physical clock is sampled.
    pub fn step(&mut self, payload: f64) -> Result<()> {
        self.validate()?;
        bounded_float(payload, 1e12)?;
        if self.n == MAX_N {
            return Err(Error::Limit);
        }
        self.step_validated(payload);
        Ok(())
    }

    fn step_validated(&mut self, payload: f64) {
        self.n += 1;
        self.roots.push(Root {
            value: payload,
            memory_count: 1,
            phase_sum: std::array::from_fn(|i| {
                [libm::cos(self.phases[i]), libm::sin(self.phases[i])]
            }),
        });
        self.fold_roots();
        let now = self.n as f64 * self.fixed_dt;
        let previous = (self.n - 1) as f64 * self.fixed_dt;
        let elapsed = now - previous;
        let dt = (if elapsed > 0.0 { elapsed } else { 0.05 }).clamp(0.005, 0.25);
        self.kuramoto(dt);
        self.think(dt, wrap(now));
        for (i, row) in self.rings.iter_mut().enumerate() {
            let mut next = Vec::with_capacity(self.width);
            for at in 0..self.width {
                let l = row[(at + self.width - 1) % self.width];
                let c = row[at];
                let r = row[(at + 1) % self.width];
                let bit = match i / 2 {
                    0 => (l & (1 - c)) | (c & r),
                    1 => (c ^ r) | (c & (1 - l)),
                    _ => l ^ (c | r),
                };
                next.push(bit);
            }
            *row = next;
        }
    }

    fn fold_roots(&mut self) {
        let count = fold_size(self.n, self.roots.len());
        if count > 0 {
            let start = self.roots.len() - count;
            let mut folded = Root {
                value: 0.0,
                memory_count: 0,
                phase_sum: [[0.0; 2]; 4],
            };
            for root in &self.roots[start..] {
                folded.value += root.value;
                folded.memory_count += root.memory_count;
                for (dst, src) in folded.phase_sum.iter_mut().zip(&root.phase_sum) {
                    dst[0] += src[0];
                    dst[1] += src[1];
                }
            }
            folded.value /= count as f64;
            self.roots.truncate(start);
            self.roots.push(folded);
        }
        if self.roots.len() > MAX_ROOTS {
            self.roots.drain(..self.roots.len() - MAX_ROOTS);
        }
    }

    fn kuramoto(&mut self, dt: f64) {
        let (magnitude, angle) = field(&self.phases);
        let dt = dt.clamp(1e-4, 0.5);
        for (phase, omega) in self.phases.iter_mut().zip(self.omegas) {
            let coupling = if magnitude > 1e-12 {
                self.kuramoto_k * libm::sin(angle - *phase)
            } else {
                0.0
            };
            *phase = wrap(*phase + (omega + coupling) * dt);
        }
    }

    fn think(&mut self, dt: f64, clock: f64) {
        let mut memory = [[0.0; 2]; 4];
        let mut count = 0u64;
        for root in &self.roots {
            count += root.memory_count;
            for (dst, src) in memory.iter_mut().zip(&root.phase_sum) {
                dst[0] += src[0];
                dst[1] += src[1];
            }
        }
        let rotation = [libm::cos(clock), libm::sin(clock)];
        for component in &mut memory {
            let re = component[0] / count as f64;
            let im = component[1] / count as f64;
            *component = [
                re * rotation[0] - im * rotation[1],
                re * rotation[1] + im * rotation[0],
            ];
        }
        let (magnitude, angle) = field(&self.phases);
        let gain = if self.memory_enabled {
            self.memory_gain
        } else {
            0.0
        };
        for (i, phase) in self.phases.iter_mut().enumerate() {
            let intrinsic = if magnitude > 1e-12 {
                self.k_coupling * libm::sin(angle - *phase)
            } else {
                0.0
            };
            let memory = memory[i % 4];
            let pull = memory[0] * -libm::sin(*phase) + memory[1] * libm::cos(*phase);
            *phase = wrap(*phase + (intrinsic + gain * pull) * dt);
        }
    }
}

/// Replay a bounded recipe, returning canonical state or full-state trace bytes.
/// `output_limit` is independent of the eventual residual target's length.
pub fn replay(checkpoint: &[u8], recipe: &Recipe, output_limit: usize) -> Result<Vec<u8>> {
    if output_limit > MAX_OUTPUT {
        return Err(Error::Limit);
    }
    recipe.validate()?;
    let mut state = State::decode(checkpoint)?;
    if state
        .n
        .checked_add(recipe.steps as u64)
        .ok_or(Error::Limit)?
        > MAX_N
    {
        return Err(Error::Limit);
    }
    let work = (6 * state.width as u64 + 16 + 81 * 4)
        .checked_mul(recipe.steps as u64)
        .ok_or(Error::Limit)?;
    if work > MAX_WORK {
        return Err(Error::Limit);
    }
    // Integer-only shape preflight precedes all oscillator and CA work.
    let required = replay_size(&state, recipe, output_limit)?;
    let mut output = Vec::new();
    if recipe.output == OutputKind::Trace {
        // One bounded reservation prevents geometric Vec growth from briefly
        // retaining both old and oversized new buffers beyond admission budget.
        output
            .try_reserve_exact(required)
            .map_err(|_| Error::Limit)?;
        output.extend_from_slice(TRACE_MAGIC);
    }
    for _ in 0..recipe.steps {
        state.step_validated(recipe.payload);
        if recipe.output == OutputKind::Trace {
            let encoded = state.encode()?;
            let next_len = output
                .len()
                .checked_add(4)
                .and_then(|n| n.checked_add(encoded.len()))
                .ok_or(Error::Limit)?;
            if next_len > required {
                return Err(Error::Integrity);
            }
            output.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
            output.extend_from_slice(&encoded);
        }
    }
    if recipe.output == OutputKind::FinalCheckpoint {
        output = state.encode()?;
    }
    if output.len() != required {
        return Err(Error::Integrity);
    }
    Ok(output)
}

// Called with validated dimensions and at most MAX_ROOTS retained roots.
fn checkpoint_size(width: usize, roots: usize) -> usize {
    CHECKPOINT_FIXED_BYTES + roots * ROOT_BYTES + 6 * width.div_ceil(8)
}

// Shared by real folding and the size-only simulation, preserving the observed
// min(root_count, 3^v3(N)) rule and its at-least-three-root outer condition.
fn fold_size(n: u64, roots: usize) -> usize {
    if !n.is_multiple_of(3) || roots < 3 {
        return 0;
    }
    let mut quotient = n;
    let mut power = 1usize;
    while quotient.is_multiple_of(3) {
        quotient /= 3;
        power = power.saturating_mul(3);
    }
    roots.min(power)
}

fn replay_size(state: &State, recipe: &Recipe, limit: usize) -> Result<usize> {
    let mut roots = state.roots.len();
    let mut length = TRACE_MAGIC.len();
    for step in 1..=recipe.steps {
        roots += 1;
        let folded = fold_size(state.n + u64::from(step), roots);
        if folded > 0 {
            roots = roots - folded + 1;
        }
        roots = roots.min(MAX_ROOTS);
        if recipe.output == OutputKind::Trace {
            length = length
                .checked_add(4)
                .and_then(|n| n.checked_add(checkpoint_size(state.width, roots)))
                .ok_or(Error::Limit)?;
            if length > limit {
                return Err(Error::Limit);
            }
        }
    }
    if recipe.output == OutputKind::FinalCheckpoint {
        length = checkpoint_size(state.width, roots);
    }
    if length > limit {
        return Err(Error::Limit);
    }
    Ok(length)
}

fn field(phases: &[f64; 16]) -> (f64, f64) {
    let mut re = 0.0;
    let mut im = 0.0;
    for &phase in phases {
        re += libm::cos(phase);
        im += libm::sin(phase);
    }
    re /= 16.0;
    im /= 16.0;
    (libm::hypot(re, im), libm::atan2(im, re))
}

fn wrap(value: f64) -> f64 {
    let remainder = libm::fmod(value, TAU);
    let wrapped = if remainder < 0.0 {
        remainder + TAU
    } else {
        remainder
    };
    // Keep one canonical zero and exclude a rounded-up representation of TAU.
    if wrapped == 0.0 || wrapped >= TAU {
        0.0
    } else {
        wrapped
    }
}

fn bounded_float(value: f64, magnitude: f64) -> Result<()> {
    if !value.is_finite() || value.abs() > magnitude {
        Err(Error::Malformed)
    } else {
        Ok(())
    }
}

fn put_float(bytes: &mut Vec<u8>, value: f64) {
    bytes.extend_from_slice(
        &(if value == 0.0 { 0.0 } else { value })
            .to_bits()
            .to_le_bytes(),
    );
}

fn read_float(r: &mut Reader<'_>) -> Result<f64> {
    let bits = r.u64()?;
    let value = f64::from_bits(bits);
    if !value.is_finite() || bits == (-0.0f64).to_bits() {
        return Err(Error::Malformed);
    }
    Ok(value)
}
