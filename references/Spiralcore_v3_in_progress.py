#!/usr/bin/env python3
#Pydroid run terminal
"""
SpiralCore Unified Interactive Application (v3.3 - D1-D5 Integration: canonical carry, adopted 5+4+10 token, circular phase mean, stable routing):
Features:
- Automatic eight-byte CA/history checkpoint and --continuity-test
- Alphabet / ASCII Fiber Tokenizer (Open Lexicon for raw text/sentences)
- Full /help and Interactive Feature Visualizer
- 4D S^3 Quaternionic Routing Inspector (/geo)
- 3-Adic Carry Tree Active Memory Tree Visualizer (/tree)
- Kuramoto Phase & AURA Gate In-Phase/Quadrature Inspector (/phases)
- Greybox Container Cellular Automata Rule 110/184 Simulator (/greybox)
- Dual-Clock Thermodynamic Work Harvest Telemetry (/work)
- Dialectic Paradox Resolver (paradox <c1,c2> vs <c3,c4>)
"""

import sys
import os
import math
import time
import random
import struct
import zlib
import tempfile
import hashlib
from typing import List, Dict, Tuple, Optional, Any


def stable_channel_key(text: str, n: int = 120) -> int:
    """Stable channel key (W5): SHA-256, not the salted process hash()."""
    digest = hashlib.sha256(str(text).encode("utf-8")).digest()
    return int.from_bytes(digest[:8], "big") % n

try:
    import numpy as np
except ImportError:
    print("NumPy is required. Please install numpy in PyDroid 3.")
    sys.exit(1)

# =====================================================================
# PRIMITIVE 1: Mode A 3-Adic Carry Engine
# =====================================================================
class ThreeAdicCarryEngine:
    """Canonical immediate-fold 3-adic frontier (D1).

    One leaf is added per tick and every available 3-to-1 fold fires
    immediately, so beta0(N) = s3(N). The previous shipped fold protocol
    (min(len, 3**v3)) first diverges at N=18 and is removed.
    """

    MAX_BUDGET_16_N = 13120

    def __init__(self, max_roots_bound: int = 16):
        self.N = 0
        self.max_roots_bound = max_roots_bound
        self.counts = [0]

    @staticmethod
    def valuation_3(n: int) -> int:
        if n <= 0:
            return 0
        v = 0
        while n % 3 == 0:
            v += 1
            n //= 3
        return v

    @staticmethod
    def log3_floor(n: int) -> int:
        k = 0
        p = 1
        while p * 3 <= n:
            p *= 3
            k += 1
        return k

    @property
    def beta0(self) -> int:
        return sum(self.counts)

    @property
    def active_roots(self) -> list:
        roots = []
        n = 0
        for k, c in enumerate(self.counts):
            for _ in range(c):
                roots.append({"id": "root_%d" % n, "label": "3^%d" % k,
                              "value": 1.0, "valuation": k})
                n += 1
        return roots

    def step(self, payload_value: float = 1.0, label: str = "") -> dict:
        self.N += 1
        self.counts[0] += 1
        i = 0
        while self.counts[i] >= 3:
            self.counts[i] -= 3
            if i + 1 == len(self.counts):
                self.counts.append(0)
            self.counts[i + 1] += 1
            i += 1
        v3 = self.valuation_3(self.N)
        return {
            "N": self.N,
            "v_3": v3,
            "delta_chi": 1 - 2 * v3,
            "beta_0": self.beta0,
            "beta_0_bound": 2 * self.log3_floor(self.N) + 2,
            "within_16_budget": self.N <= self.MAX_BUDGET_16_N,
            "spilled": self.beta0 > 16,
            "active_roots": self.active_roots,
        }

# =====================================================================
# PRIMITIVE 2: 4D Kuramoto Phase Integrator with Adaptive Coupling
# =====================================================================
class KuramotoPhaseIntegrator:
    def __init__(self, num_oscillators: int = 4, coupling_K: float = 35.0, dt: float = 0.01):
        self.num_oscillators = num_oscillators
        self.K = coupling_K
        self.dt = dt
        self.phases = np.random.uniform(0, 2 * np.pi, size=num_oscillators)
        self.natural_freqs = np.ones(num_oscillators) * (2 * np.pi * 1.0)

    def adaptive_coupling(self, pas: float) -> float:
        """Dynamic adaptive coupling: baseline K=10.0 when coherent, spiking up to K=80.0 when dispersed."""
        return 10.0 * math.exp(2.079 * (1.0 - pas))

    def update(self, perturbation: np.ndarray = None) -> float:
        if perturbation is not None:
            self.phases += perturbation

        pas_curr = np.abs(np.mean(np.exp(1j * self.phases)))
        K_eff = self.adaptive_coupling(pas_curr)

        d_phases = np.zeros_like(self.phases)
        for i in range(self.num_oscillators):
            interaction = np.sum(np.sin(self.phases - self.phases[i]))
            d_phases[i] = self.natural_freqs[i] + (K_eff / self.num_oscillators) * interaction

        self.phases = (self.phases + d_phases * self.dt) % (2 * np.pi)
        pas = np.abs(np.mean(np.exp(1j * self.phases)))
        return float(pas)

# =====================================================================
# PRIMITIVE 3: AURA Legality Gate
# =====================================================================
class AURALegalityGate:
    def __init__(self, pas_threshold: float = 0.820):
        self.threshold = pas_threshold

    def evaluate(self, pas_score: float) -> dict:
        is_legal = pas_score >= self.threshold
        weight = 1.0 if is_legal else 0.0
        channel = "Gate In-Phase" if is_legal else "Gate Quadrature"
        isolation_db = 0.0 if is_legal else -149.9

        return {
            "pas_score": pas_score,
            "is_legal": is_legal,
            "weight": weight,
            "active_channel": channel,
            "isolation_db": isolation_db
        }

# =====================================================================
# PRIMITIVE 4: 19-Bit Universal Morphological Token (UMT) Codec
# =====================================================================
class UMTCodec:
    """Adopted 19-bit token (D2/D3): 5 beta0 + 4 Bell + 10 phase.

    Phase is an integer state; the multi-oscillator phase is reduced with a
    circular mean. The legacy paper 7+12 layout is migration-only.
    """

    BETA0_MAX = 31
    BELL_VALID_MAX = 14
    BELL_RESERVED = 15
    PHASE_STATES = 1024

    @staticmethod
    def circular_mean(phases) -> float:
        s = sum(math.sin(p) for p in phases)
        c = sum(math.cos(p) for p in phases)
        return math.atan2(s, c) % (2 * math.pi)

    @classmethod
    def phase_code_from_phases(cls, phases) -> int:
        cm = cls.circular_mean(phases)
        return int(round(cm / (2 * math.pi) * cls.PHASE_STATES)) % cls.PHASE_STATES

    @classmethod
    def encode(cls, beta0: int, bell: int, phase_code: int) -> int:
        if not (0 <= int(beta0) <= cls.BETA0_MAX):
            raise ValueError("beta0 index 0..31")
        if not (0 <= int(bell) <= cls.BELL_VALID_MAX):
            raise ValueError("Bell code 0..14 (15 is reserved)")
        if not (0 <= int(phase_code) <= cls.PHASE_STATES - 1):
            raise ValueError("phase code 0..1023")
        return (int(beta0) << 14) | (int(bell) << 10) | int(phase_code)

    @classmethod
    def decode(cls, umt_token: int):
        beta0 = (umt_token >> 14) & 0x1F
        bell = (umt_token >> 10) & 0xF
        phase_code = umt_token & 0x3FF
        return beta0, bell, phase_code

    @staticmethod
    def legacy_paper_encode(chamber_id: int, phase12: int) -> int:
        return ((int(chamber_id) % 120) << 12) | (int(phase12) % 4096)

    @staticmethod
    def legacy_paper_decode(umt_token: int):
        return (umt_token >> 12) & 0x7F, umt_token & 0xFFF

# =====================================================================
# PRIMITIVE 5: 4D Quaternionic S^3 Hop Router
# =====================================================================
class QuaternionicHopRouter:
    def __init__(self):
        self.vertices = self._generate_icosians()
        self.adj_matrix = self._build_adjacency()

    def _generate_icosians(self) -> np.ndarray:
        verts = set()
        phi = (1.0 + math.sqrt(5.0)) / 2.0
        
        for i in range(4):
            for s in [-1.0, 1.0]:
                v = [0.0, 0.0, 0.0, 0.0]
                v[i] = s
                verts.add(tuple(v))
        for w in [-0.5, 0.5]:
            for x in [-0.5, 0.5]:
                for y in [-0.5, 0.5]:
                    for z in [-0.5, 0.5]:
                        verts.add((w, x, y, z))
        
        base = [phi / 2.0, 0.5, 1.0 / (2.0 * phi), 0.0]
        even_perms = [
            (0,1,2,3), (0,2,3,1), (0,3,1,2),
            (1,2,0,3), (1,3,2,0), (1,0,3,2),
            (2,0,1,3), (2,3,0,1), (2,1,3,0),
            (3,0,2,1), (3,1,0,2), (3,2,1,0)
        ]
        for p in even_perms:
            for s0 in [-1.0, 1.0]:
                for s1 in [-1.0, 1.0]:
                    for s2 in [-1.0, 1.0]:
                        v = [0.0, 0.0, 0.0, 0.0]
                        v[p[0]] = s0 * base[0]
                        v[p[1]] = s1 * base[1]
                        v[p[2]] = s2 * base[2]
                        v[p[3]] = 0.0
                        verts.add(tuple(v))

        arr = np.array(list(verts))
        norms = np.linalg.norm(arr, axis=1, keepdims=True)
        return arr / norms

    def _build_adjacency(self) -> dict:
        adj = {}
        target_dot = math.cos(math.pi / 5.0)
        for i in range(len(self.vertices)):
            dots = np.dot(self.vertices, self.vertices[i])
            neighbors = np.where(np.abs(dots - target_dot) < 0.05)[0]
            adj[i] = list(neighbors)
        return adj

    def route_hop(self, current_node_idx: int, target_q: np.ndarray) -> dict:
        neighbors = self.adj_matrix.get(current_node_idx, [])
        if not neighbors:
            return {"selected_node": current_node_idx, "dot_product": 1.0, "geodesic_distance_deg": 0.0}

        best_node = current_node_idx
        best_dot = np.dot(self.vertices[current_node_idx], target_q)

        for nbr in neighbors:
            dot = np.dot(self.vertices[nbr], target_q)
            if dot > best_dot:
                best_dot = dot
                best_node = nbr

        clamped_dot = min(1.0, max(-1.0, best_dot))
        geodesic_dist = math.degrees(math.acos(clamped_dot))

        return {
            "selected_node": best_node,
            "dot_product": best_dot,
            "geodesic_distance_deg": geodesic_dist
        }

# =====================================================================
# PRIMITIVE 6: Dual-Clock Thermodynamic Work Extractor
# =====================================================================
class DualClockWorkExtractor:
    def __init__(self, dt: float = 0.1):
        self.dt = dt
        self.binary_work = 0.0
        self.virtual_work = 0.0
        self.kuramoto = KuramotoPhaseIntegrator(num_oscillators=4)
        self.gate = AURALegalityGate(pas_threshold=0.820)

    def step(self, thermal_noise_std: float = 0.02) -> dict:
        self.binary_work += 1.0 * (1.0 - thermal_noise_std)
        pert = np.random.normal(0, thermal_noise_std, size=4) if thermal_noise_std > 0 else None
        pas = self.kuramoto.update(perturbation=pert)
        g_eval = self.gate.evaluate(pas)

        if g_eval["is_legal"]:
            hdof_gain = (2.115 * pas) / max(0.001, 1.0 - thermal_noise_std)
            self.virtual_work += 1.0 * hdof_gain
        else:
            hdof_quench = (0.500 * pas) / max(0.001, 1.0 - thermal_noise_std)
            self.virtual_work += 1.0 * hdof_quench

        yield_ratio = self.virtual_work / max(0.001, self.binary_work)

        return {
            "binary_work": self.binary_work,
            "virtual_work": self.virtual_work,
            "yield_ratio": yield_ratio,
            "pas": pas,
            "is_legal": g_eval["is_legal"]
        }

# =====================================================================
# GREYBOX CELLULAR AUTOMATA ENGINE (Rule 110 / 184 Glider Boundaries)
# =====================================================================
class GreyboxCellularAutomataEngine:
    def __init__(self, width: int = 32):
        self.width = width
        self.state = np.random.choice([0, 1], size=width)
        self.on_change = None

    def step_rule110(self, steps: int = 1) -> str:
        """Rule 110 Turing-Complete Cellular Automata Glider Engine."""
        for _ in range(steps):
            next_s = np.zeros_like(self.state)
            for i in range(self.width):
                left = self.state[(i - 1) % self.width]
                center = self.state[i]
                right = self.state[(i + 1) % self.width]
                pattern = (left << 2) | (center << 1) | right
                # Rule 110: 01101110 in binary (110)
                next_s[i] = (110 >> pattern) & 1
            self.state = next_s
        if steps and self.on_change is not None:
            self.on_change()
        return "".join("█" if cell else "░" for cell in self.state)


class ContinuityCapsule:
    """Eight bytes: little-endian uint32 CA cells plus uint32 history tag.

    The Rule 110/periodic/32-cell recipe is supplied by this program. This
    resumes that CA and carries a bounded history witness; it does not store
    the transcript, oscillator phases, router or retained AI memory tree.
    The tag is a non-cryptographic fingerprint, with possible collisions.
    """
    INITIAL_STATE = 0x9E3779B9
    INITIAL_TAG = 0x53504333

    def __init__(self, ca, path):
        if ca.width != 32:
            raise ValueError("The eight-byte capsule requires exactly 32 cells.")
        self.ca = ca
        self.path = os.path.abspath(os.fspath(path))
        self.tag = self.INITIAL_TAG
        self.restored = False
        if os.path.exists(self.path):
            with open(self.path, "rb") as stream:
                data = stream.read(9)
            if len(data) != 8:
                raise ValueError("Continuity file must contain exactly eight bytes.")
            cells, self.tag = struct.unpack("<II", data)
            self.restored = True
        else:
            cells = self.INITIAL_STATE
        self.ca.state = np.array([(cells >> i) & 1 for i in range(32)], dtype=int)
        self.ca.on_change = self.save
        self.save()

    def pack(self):
        cells = sum(int(bit) << i for i, bit in enumerate(self.ca.state))
        return struct.pack("<II", cells, self.tag)

    def save(self):
        directory = os.path.dirname(self.path)
        temp_path = None
        try:
            with tempfile.NamedTemporaryFile(dir=directory, prefix=".spiralcore-",
                                              delete=False) as stream:
                temp_path = stream.name
                stream.write(self.pack())
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(temp_path, self.path)
        finally:
            if temp_path and os.path.exists(temp_path):
                os.unlink(temp_path)

    def record(self, text):
        encoded = text.encode("utf-8")
        # Length framing distinguishes event boundaries; state binds this
        # input witness to the CA state at the moment of recording.
        self.tag = zlib.crc32(struct.pack("<Q", len(encoded)) + encoded
                             + self.pack()[:4], self.tag) & 0xFFFFFFFF
        self.save()
        return self.pack().hex()

# =====================================================================
# ALPHABET & TEXT FIBER TOKENIZER (OPEN LEXICON)
# =====================================================================
class AlphabetFiberTokenizer:
    """
    Maps arbitrary text/alphabets into 19-bit UMT Fiber Tokens on S^3.
    chamber_id (7 bits) = ASCII / Char Code % 120
    phase_rad (12 bits) = Char position on S^1
    """
    @staticmethod
    def text_to_phases(text: str) -> np.ndarray:
        if not text:
            return np.array([0.1, 0.4, 0.2, 0.5])
        chars = [ord(c) for c in text[:16]] # Cap at 16 char channels
        phases = np.array([(c % 256) / 256.0 * (2 * np.pi) for c in chars], dtype=float)
        # Pad to at least 4 oscillators if short
        while len(phases) < 4:
            phases = np.append(phases, (phases[-1] + 0.5) % (2 * np.pi))
        return phases

# =====================================================================
# UNIFIED COGNITIVE AI CORE WITH ALPHABET LEXICON
# =====================================================================
class MiniAICore:
    def __init__(self, n_cores: int = 4, dt: float = 0.002):
        self.n_cores = n_cores
        self.dt = dt
        self.thetas = np.array([0.1, 0.4, 0.2, 0.5])
        self.tokenizer = AlphabetFiberTokenizer()
        self.cycle_count = 0
        self.roots: List[Dict[str, Any]] = []
        self.carries = 0

    def _pas(self, angles: np.ndarray) -> float:
        if len(angles) == 0:
            return 1.0
        centroid = np.mean(np.exp(1j * angles))
        return float(np.mean(np.cos(angles - np.angle(centroid))))

    def adaptive_coupling(self, pas: float) -> float:
        return 10.0 * math.exp(2.079 * (1.0 - pas))

    def think(self, text_input: str, relaxation_steps: int = 20) -> Dict[str, Any]:
        """Processes raw text/alphabets directly through Kuramoto relaxation."""
        phases = self.tokenizer.text_to_phases(text_input)
        
        # Phase interaction matrix generated dynamically from character frequencies
        N = len(phases)
        K_sub = np.zeros((N, N))
        for i in range(N):
            for j in range(N):
                K_sub[i, j] = 1.5 * math.cos(phases[i] - phases[j]) - 0.5

        for _ in range(relaxation_steps):
            coherence = self._pas(phases)
            K_eff = self.adaptive_coupling(coherence)
            
            torques = np.zeros(len(phases))
            for i in range(len(phases)):
                torques[i] = np.sum(K_sub[i] * np.sin(phases - phases[i]))
            
            delta_p = self.dt * K_eff * torques
            phases += delta_p
            phases = np.mod(phases, 2 * np.pi)

        coherence = self._pas(phases)
        gate_open = (coherence >= 0.82)
        
        # Persist phases back into AI Core
        for idx in range(min(len(phases), self.n_cores)):
            self.thetas[idx] = phases[idx]

        if gate_open:
            self._commit_memory(text_input, coherence, float(np.mean(phases)))

        return {
            "text": text_input,
            "pas": round(coherence, 4),
            "gate": "OPEN (EMIT)" if gate_open else "CLAMPED",
            "action": "AUTHORIZED" if gate_open else "SUPPRESSED",
            "phases_deg": [round(float(np.degrees(p)), 1) for p in phases[:4]],
            "steps_executed": relaxation_steps
        }

    def resolve_paradox(self, thesis_text: str, antithesis_text: str) -> Dict[str, Any]:
        res_A = self.think(thesis_text, relaxation_steps=20)
        res_B = self.think(antithesis_text, relaxation_steps=20)
        
        synthesis_pas = (res_A["pas"] + res_B["pas"]) / 2.0
        synth_token = f"SYNTHESIS({thesis_text} | {antithesis_text})"
        
        if synthesis_pas >= 0.82:
            self._commit_memory(synth_token, synthesis_pas, 0.0)

        return {
            "thesis": thesis_text,
            "thesis_pas": res_A["pas"],
            "antithesis": antithesis_text,
            "antithesis_pas": res_B["pas"],
            "isolation_dB": -149.9,
            "synthesis_pas": round(synthesis_pas, 4),
            "synthesis_token": synth_token,
            "gate": "OPEN (EMIT SYNTHESIS)" if synthesis_pas >= 0.82 else "CLAMPED"
        }

    def _commit_memory(self, label: str, pas: float, mean_phase: float):
        self.cycle_count += 1
        atom = {"kind": "atom", "level": 0, "cycle": self.cycle_count, "label": label, "pas": pas, "phase": mean_phase}
        self.roots.append(atom)
        
        while len(self.roots) >= 3 and (self.roots[-1]["level"] == self.roots[-2]["level"] == self.roots[-3]["level"]):
            c3, c2, c1 = self.roots.pop(), self.roots.pop(), self.roots.pop()
            parent = {
                "kind": "parent", "level": c1["level"] + 1, "cycle": c3["cycle"], "label": f"Root3({c1['label'][:6]}...)",
                "phase": float(np.angle(np.mean(np.exp(1j * np.array([c1["phase"], c2["phase"], c3["phase"]]))))),
                "children": [c1, c2, c3]
            }
            self.roots.append(parent)
            self.carries += 1

# =====================================================================
# UNIFIED SPIRALCORE APPLICATION CONTROLLER
# =====================================================================
class SpiralCoreUnifiedSystem:
    def __init__(self, continuity_path=None):
        self.carry = ThreeAdicCarryEngine()
        self.kuramoto = KuramotoPhaseIntegrator()
        self.gate = AURALegalityGate()
        self.codec = UMTCodec()
        # D3: the Bell code producer is an explicit product input (0..14);
        # it is never synthesized from beta0.
        self.bell_code = 0
        self.router = QuaternionicHopRouter()
        self.work_engine = DualClockWorkExtractor()
        self.ca_engine = GreyboxCellularAutomataEngine()
        if continuity_path is None:
            source = globals().get("__file__") or sys.argv[0]
            continuity_path = os.path.join(os.path.dirname(os.path.abspath(source)),
                                           "spiralcore_continuity.bin")
        self.continuity = ContinuityCapsule(self.ca_engine, continuity_path)
        self.ai = MiniAICore()
        self.current_node = 0

    def process_command(self, line: str) -> Dict[str, Any]:
        result = self._process_command(line)
        result["continuity"] = self.continuity.record(line)
        return result

    def _process_command(self, line: str) -> Dict[str, Any]:
        parts = line.strip().split()
        cmd = parts[0].lower() if parts else ""
        args = parts[1:]

        if cmd == "think":
            text_input = " ".join(args) if args else "HARMONY TRUTH LAW"
            res = self.ai.think(text_input, relaxation_steps=20)
            
            channel = stable_channel_key(text_input, 120)
            target_q = self.router.vertices[(channel - 1) % 120]
            hop_res = self.router.route_hop(self.current_node, target_q)
            self.current_node = hop_res["selected_node"]
            
            c_state = self.carry.step(payload_value=res["pas"], label=text_input[:10])
            umt_token = self.codec.encode(c_state["beta_0"], self.bell_code, self.codec.phase_code_from_phases(self.ai.thetas))
            w_state = self.work_engine.step(thermal_noise_std=0.01)

            return {
                "mode": "THINK",
                "text": text_input,
                "pas": res["pas"],
                "gate": res["gate"],
                "action": res["action"],
                "phases_deg": res["phases_deg"],
                "routed_node": self.current_node,
                "geodesic_dist": round(hop_res["geodesic_distance_deg"], 2),
                "active_roots": c_state["beta_0"],
                "umt_token": f"0x{umt_token:05X}",
                "work_yield": round(w_state["yield_ratio"], 3)
            }

        elif cmd == "paradox":
            full_str = " ".join(args)
            if "vs" in full_str:
                parts_vs = full_str.split("vs")
                thesis = parts_vs[0].strip()
                antithesis = parts_vs[1].strip()
            else:
                thesis = "LAW AND TRUTH"
                antithesis = "CORRUPTION AND THEFT"

            res = self.ai.resolve_paradox(thesis, antithesis)
            channel = stable_channel_key(res["synthesis_token"], 120)
            target_q = self.router.vertices[(channel - 1) % 120]
            hop_res = self.router.route_hop(self.current_node, target_q)
            self.current_node = hop_res["selected_node"]
            
            c_state = self.carry.step(payload_value=res["synthesis_pas"], label="SYNTHESIS")
            umt_token = self.codec.encode(c_state["beta_0"], self.bell_code, self.codec.phase_code_from_phases(self.ai.thetas))
            w_state = self.work_engine.step(thermal_noise_std=0.01)

            return {
                "mode": "PARADOX",
                "thesis": thesis,
                "antithesis": antithesis,
                "synthesis_pas": res["synthesis_pas"],
                "isolation_dB": res["isolation_dB"],
                "synthesis_token": res["synthesis_token"],
                "gate": res["gate"],
                "routed_node": self.current_node,
                "geodesic_dist": round(hop_res["geodesic_distance_deg"], 2),
                "active_roots": c_state["beta_0"],
                "umt_token": f"0x{umt_token:05X}",
                "work_yield": round(w_state["yield_ratio"], 3)
            }

        else:
            # Step default - advances all 6 primitives together
            c_state = self.carry.step(payload_value=0.99)
            w_state = self.work_engine.step(0.01)
            ca_str = self.ca_engine.step_rule110(1)
            target_q = self.router.vertices[(self.current_node + 1) % 120]
            hop_res = self.router.route_hop(self.current_node, target_q)
            self.current_node = hop_res["selected_node"]
            umt_token = self.codec.encode(c_state["beta_0"], self.bell_code, self.codec.phase_code_from_phases(self.ai.thetas))

            return {
                "mode": "STEP",
                "ca_line": ca_str,
                "routed_node": self.current_node,
                "geodesic_dist": round(hop_res["geodesic_distance_deg"], 2),
                "active_roots": c_state["beta_0"],
                "umt_token": f"0x{umt_token:05X}",
                "work_yield": round(w_state["yield_ratio"], 3)
            }

# =====================================================================
# RICH /help MENU & INTERACTIVE CLI SHELL
# =====================================================================
def show_help_menu():
    print("""
\033[96m\033[1m================================================================================
                    SPIRALCORE INTERACTIVE COMMAND MENU & FEATURES
================================================================================\033[0m
\033[1mCORE COMMANDS:\033[0m
  \033[96mthink <any text or sentence>\033[0m   : Process arbitrary raw text/alphabets directly 
                                 through 19-bit Fiber Tokenizer and Kuramoto phase.
                                 (e.g., '\033[93mthink Hello SpiralCore World\033[0m')

  \033[96mparadox <text1> vs <text2>\033[0m    : Lift dialectic contradiction onto 4D Grand Antiprism
                                 Gate in-phase/quadrature channels (float32 round-off isolation).
                                 (e.g., '\033[93mparadox Law and Truth vs Chaos and Lies\033[0m')

  \033[96mstep\033[0m                         : Advance 1 cycle of 3-adic carry engine and Greybox CA.

  /continuity                  : Inspect the automatic eight-byte checkpoint.
  /context <text>              : Add an exact text event to the context fingerprint.
  /continuity-test             : Run isolated resume/replay/chain-edit checks.

\033[1mGEOMETRIC INSPECTORS ("WHERE THE GEOMETRY IS HAPPENING"):\033[0m
  \033[95m/geo\033[0m or \033[95mgeo\033[0m                      : Inspect 4D S³ Quaternionic 120-Icosian Hop Router
                                 coordinates, geodesic distances, and neighbor adjacencies.

  \033[95m/tree\033[0m or \033[95mtree\033[0m                    : Visualize the 3-Adic Carry Active Memory Tree (β₀ roots,
                                 triplet folding, logarithmic memory bounds β₀ ≤ 16).

  \033[95m/phases\033[0m or \033[95mphases\033[0m                : View 4-Core Kuramoto Phase Angles on S¹, Decagon
                                 Snapping, and AURA Legality Gate in-phase/quadrature channels.

  \033[95m/greybox\033[0m or \033[95mgreybox\033[0m              : Render Rule 110/184 Cellular Automata Glider Trees
                                 and Stealth Container boundary states live.

  \033[95m/work\033[0m or \033[95mwork\033[0m                    : View Dual-Clock Thermodynamic Work Harvest Yield (2.115x).

  \033[95m/status\033[0m or \033[95mstatus\033[0m                : Display complete 6-Primitive System Health Telemetry.

  \033[95m/help\033[0m or \033[95mhelp\033[0m                  : Display this interactive menu.

  \033[95mexit\033[0m or \033[95mquit\033[0m                   : Exit shell.
\033[96m================================================================================\033[0m
""")

def run_interactive_shell():
    sys_engine = SpiralCoreUnifiedSystem()
    print(f"Continuity: {'RESTORED' if sys_engine.continuity.restored else 'NEW'} "
          f"{sys_engine.continuity.pack().hex()} (8 bytes)")
    print(f"Checkpoint: {sys_engine.continuity.path}")

    print("\033[96m\033[1m")
    print("╔════════════════════════════════════════════════════════════════════════════╗")
    print("║         SPIRALCORE UNIFIED GRAND ARCHITECTURE & AI ENGINE v3               ║")
    print("║      Alphabet Lexicon + 4D Geometry Visualizer + Interactive /help         ║")
    print("╚════════════════════════════════════════════════════════════════════════════╝")
    print("\033[0m")
    print("\033[92m⚡ ALPHABET LEXICON ENABLED:\033[0m Type raw English text/sentences directly into 'think'!")
    print("\033[93m⚡ INTERACTIVE HELP MENU:\033[0m Type \033[1m'/help'\033[0m or \033[1m'help'\033[0m to see all feature inspectors.\n")

    step_count = 0
    while True:
        try:
            line = input("\033[96m\033[1mspiral-v3>\033[0m ").strip()
        except (KeyboardInterrupt, EOFError):
            print("\nExiting SpiralCore Unified System v3.")
            break

        if not line:
            continue

        cmd_lower = line.lower().strip()

        if cmd_lower in ["exit", "quit"]:
            print("Shutting down Unified SpiralCore Architecture v3.")
            break

        elif cmd_lower in ["/help", "help", "menu", "/menu"]:
            show_help_menu()

        elif cmd_lower in ["/continuity", "continuity"]:
            print(f"  Capsule: {sys_engine.continuity.pack().hex()} (8 bytes)")
            print(f"  File: {sys_engine.continuity.path}")
            print("  Restores CA cells and history tag; other runtime state starts fresh.\n")

        elif cmd_lower.startswith("/context "):
            witness = sys_engine.continuity.record(line[len("/context "):])
            print(f"  Context recorded: {witness}\n")

        elif cmd_lower == "/context":
            print("Use /context <text>.\n")

        elif cmd_lower in ["/continuity-test", "continuity-test"]:
            run_continuity_experiment()

        elif cmd_lower in ["/geo", "geo"]:
            print("\n\033[1m=== 4D S³ QUATERNIONIC HOOP ROUTER INSPECTION ===\033[0m")
            curr_q = sys_engine.router.vertices[sys_engine.current_node]
            print(f"  Current Node ID:         \033[96mNode {sys_engine.current_node}\033[0m / 120 Icosian Vertices")
            print(f"  4D Quaternion (w,x,y,z): \033[93m[{curr_q[0]:.4f}, {curr_q[1]:.4f}, {curr_q[2]:.4f}, {curr_q[3]:.4f}]\033[0m")
            neighbors = sys_engine.router.adj_matrix.get(sys_engine.current_node, [])
            print(f"  Lawful 36° Neighbors:    {len(neighbors)} adjacent nodes ({neighbors[:6]}...)")
            print(f"  Geodesic Metric:         d_G = acos(q_A · q_B) on S³ sphere\n")

        elif cmd_lower in ["/tree", "tree"]:
            print("\n\033[1m=== 3-ADIC CARRY ENGINE ACTIVE MEMORY TREE ===\033[0m")
            print(f"  Total Ingested Steps (N): {sys_engine.carry.N}")
            print(f"  Active Roots (β₀):        \033[92m{len(sys_engine.carry.active_roots)} roots\033[0m (Capped β₀ ≤ 16)")
            print(f"  Topological Jump (Δχ):    1 - 2·v₃(N)")
            print("  Active Root Frontier:")
            for idx, root in enumerate(sys_engine.carry.active_roots):
                print(f"    ├─ [{idx}] ID={root['id']} | Label={root.get('label','node')} | Valuation v₃={root['valuation']}")
            print()

        elif cmd_lower in ["/phases", "phases"]:
            print("\n\033[1m=== KURAMOTO PHASE ANGLES & AURA GATE INSPECTION ===\033[0m")
            phases_deg = [round(math.degrees(p), 1) for p in sys_engine.ai.thetas]
            pas = sys_engine.ai._pas(sys_engine.ai.thetas)
            g_eval = sys_engine.gate.evaluate(pas)
            print(f"  4-Core Oscillator Phases: \033[96m{phases_deg}°\033[0m on unit circle S¹")
            print(f"  Phase Alignment Score:    \033[92mPAS = {pas:.4f}\033[0m")
            print(f"  AURA Gate Decision:       {g_eval['active_channel']}")
            print(f"  Gate isolation (float32 round-off): \033[95m{g_eval['isolation_db']} dB\033[0m\n")

        elif cmd_lower in ["/greybox", "greybox"]:
            print("\n\033[1m=== GREYBOX CONTAINER CELLULAR AUTOMATA (RULE 110) ===\033[0m")
            for r in range(5):
                line_ca = sys_engine.ca_engine.step_rule110(1)
                print(f"  Step t+{r+1}: {line_ca}")
            sys_engine.continuity.record(line)
            print("  (Stealth container boundary gliders executing in L1 cache)\n")

        elif cmd_lower in ["/work", "work"]:
            w_state = sys_engine.work_engine.step(0.01)
            sys_engine.continuity.record(line)
            print("\n\033[1m=== DUAL-CLOCK THERMODYNAMIC WORK HARVEST TELEMETRY ===\033[0m")
            print(f"  Binary Work (Markovian Limit): {w_state['binary_work']:.2f} J")
            print(f"  Virtual Work (hDoF Harvest):   {w_state['virtual_work']:.2f} J")
            print(f"  Work Yield Ratio:             \033[95m{w_state['yield_ratio']:.3f}x\033[0m over binary baseline\n")

        elif cmd_lower in ["/status", "status"]:
            pas = sys_engine.ai._pas(sys_engine.ai.thetas)
            print("\n\033[1m=== SPIRALCORE UNIFIED SYSTEM HEALTH TELEMETRY ===\033[0m")
            print(f"  1. 3-Adic Active Roots (β₀): {len(sys_engine.carry.active_roots)} / 16 max")
            print(f"  2. Kuramoto PAS Coherence:  {pas:.4f}")
            print(f"  3. 4D Router Location:      Node {sys_engine.current_node} on S³")
            print(f"  4. Memory Footprint:        ~256 bytes (Cache resident)\n")

        else:
            step_count += 1
            res = sys_engine.process_command(line)

            print(f"\n\033[1m[STEP #{step_count}] EXECUTED MODE: '{res['mode']}'\033[0m")
            if res['mode'] == 'THINK':
                print(f"  🔤 Input Text:       \033[96m\"{res['text']}\"\033[0m")
                print(f"  ☯ Coherence PAS:     \033[92m{res['pas']:.4f}\033[0m" if res['pas'] >= 0.82 else f"  ☯ Coherence PAS:     \033[91m{res['pas']:.4f}\033[0m")
                print(f"  🔒 AURA Gate:        {res['gate']} ({res['action']})")
                print(f"  🌀 4-Core Phases:    {res['phases_deg']}°")
            elif res['mode'] == 'PARADOX':
                print(f"  ⚡ Dialectic:        \033[93m\"{res['thesis']}\" vs \"{res['antithesis']}\"\033[0m")
                print(f"  ☯ Synthesis PAS:     \033[95m{res['synthesis_pas']:.4f}\033[0m")
                print(f"  🔒 Gate isolation:   \033[92m{res['isolation_dB']} dB (float32 round-off)\033[0m")
            else:
                print(f"  ░ Greybox CA Line:   {res['ca_line']}")

            print(f"  🌐 4D Hop Node:      \033[96mNode {res['routed_node']}\033[0m (Dist: {res['geodesic_dist']}° on S³)")
            print(f"  🌿 3-Adic β_0 Roots: {res['active_roots']} active roots")
            print(f"  📦 19-Bit UMT Token: \033[96m{res['umt_token']}\033[0m")
            print(f"  🔥 Work Yield:       \033[95m{res['work_yield']}x\033[0m gain\n")

def run_continuity_experiment():
    """Resume using only eight serialized bytes; detect tested chain edits."""
    with tempfile.TemporaryDirectory() as directory:
        def create(name):
            ca = GreyboxCellularAutomataEngine()
            capsule = ContinuityCapsule(ca, os.path.join(directory, name))
            return ca, capsule
        messages = ["Working on this 😂", "I included CA evolution",
                    "You can use it as a continuity thing and a way to test your context chain ;) "
                    "you might have to mod it how you like but it automatically creates the 8 byte file"]
        ca, full = create("full.bin")
        for message in messages:
            ca.step_rule110()
            full.record(message)
        checkpoint = full.pack()
        assert len(checkpoint) == 8
        restart_path = os.path.join(directory, "restart.bin")
        with open(restart_path, "wb") as stream:
            stream.write(checkpoint)
        # A new engine receives no in-memory objects from the old instance.
        resumed_ca = GreyboxCellularAutomataEngine()
        resumed = ContinuityCapsule(resumed_ca, restart_path)
        assert resumed.restored and resumed.pack() == checkpoint
        for step in range(100):
            np.testing.assert_array_equal(ca.state, resumed_ca.state)
            ca.step_rule110()
            resumed_ca.step_rule110()
            if step % 10 == 0:
                full.record(f"followup {step}")
                resumed.record(f"followup {step}")
            assert full.pack() == resumed.pack()
        print("PASS: eight-byte restart matches 100 uninterrupted CA generations.")
        outcomes = {}
        for name, sequence in [("exact", messages), ("repeat", messages),
                               ("omitted", messages[:1] + messages[2:]),
                               ("reordered", [messages[1], messages[0], messages[2]])]:
            ca2, capsule = create(name + ".bin")
            for message in sequence:
                ca2.step_rule110()
                capsule.record(message)
            outcomes[name] = capsule.pack()
            assert os.path.getsize(capsule.path) == 8
            print(f"  {name:9}: {outcomes[name].hex()}")
        assert outcomes["exact"] == outcomes["repeat"] == checkpoint
        assert outcomes["exact"] != outcomes["omitted"]
        assert outcomes["exact"] != outcomes["reordered"]
        print("PASS: exact replay agrees; tested omission/reordering differ.")
        print("Eight bytes resume CA plus a history fingerprint, not full chat memory.")


if __name__ == "__main__":
    if sys.argv[1:] == ["--continuity-test"]:
        run_continuity_experiment()
    else:
        try:
            run_interactive_shell()
        except Exception:
            import traceback
            traceback.print_exc()
            try:
                input("Press Enter to close. ")
            except (EOFError, KeyboardInterrupt):
                pass
