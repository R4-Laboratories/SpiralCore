#!/usr/bin/env python3
"""Bounded reference qualification only; no original module import or card execution."""
import __future__
import argparse
import ast
import cmath
import copy
import hashlib
import json
import math
import pathlib
import platform
import random
import struct
import sys
from types import MethodType, SimpleNamespace

SOURCE_SHA256 = '2e11825b16a825f066bc249d785d436c3345abe4fd2c0958fbdb156dea9eb269'
WRAPPER_SHA256 = '77076e195d3f93d91b79b653e946af58b215c52166382269de9360da83226ad0'
FUNCTIONS = ('valuation_3', 'encode_binary_assumption_row', 'rule_184', 'rule_110',
             'rule_30', 'step_ca', 'kuramoto_n2_field_step', 'kuramoto_step')
EXPORTED = FUNCTIONS + ('_bytes_to_bits', 'reference_default_row', 'reference_fold_prefix',
                        'memory_field', 'think', 'sync_system_to_kuramoto', 'reference_full_step')


def sha(data):
    return hashlib.sha256(data).hexdigest()


def compile_reference(raw):
    if sha(raw) != SOURCE_SHA256:
        raise ValueError('source SHA-256 does not match pinned kernel')
    source = raw.decode('utf-8')
    tree = ast.parse(source)
    selected, records = [], []

    def record(node, name, modification='none'):
        segment = ast.get_source_segment(source, node)
        records.append({'name': name, 'source_start_line': node.lineno,
                        'source_end_line': node.end_lineno, 'source_sha256': sha(segment.encode()),
                        'modification': modification, 'source': segment})

    for name in FUNCTIONS:
        matches = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == name]
        assert len(matches) == 1
        node = copy.deepcopy(matches[0])
        assert not node.decorator_list
        record(node, name)
        selected.append(node)

    codec = next(n for n in tree.body if isinstance(n, ast.ClassDef) and n.name == 'SystemLawCodec')
    method = next(n for n in codec.body if isinstance(n, ast.FunctionDef) and n.name == '_bytes_to_bits')
    record(method, '_bytes_to_bits', 'remove staticmethod decorator; preserve function body')
    byte_fn = copy.deepcopy(method)
    byte_fn.decorator_list = []
    selected.append(byte_fn)

    # Read only an expression from the original initializer; never instantiate its runtime.
    runtime = next(n for n in tree.body if isinstance(n, ast.ClassDef) and n.name == 'SpiralcoreRuntime')
    init = next(n for n in runtime.body if isinstance(n, ast.FunctionDef) and n.name == '__init__')
    row = next(n for n in init.body if isinstance(n, ast.Assign)
               and any(isinstance(t, ast.Name) and t.id == 'init_row' for t in n.targets))
    record(row, 'reference_default_row', 'wrap exact init_row expression in function(ca_width)')
    row_fn = ast.parse('def reference_default_row(ca_width):\n    return []\n').body[0]
    row_fn.body[0].value = copy.deepcopy(row.value)
    selected.append(row_fn)

    # Only the original step prefix, ending BEFORE clock sync, CA, routing, audio or autosave.
    step = next(n for n in runtime.body if isinstance(n, ast.FunctionDef) and n.name == 'step')
    cut = next(i for i, n in enumerate(step.body) if isinstance(n, ast.Assign)
               and any(isinstance(t, ast.Name) and t.id == 'dt' for t in n.targets))
    prefix = copy.deepcopy(step)
    prefix.name = 'reference_fold_prefix'
    prefix.body = prefix.body[:cut]
    prefix.end_lineno = prefix.body[-1].end_lineno
    prefix.end_col_offset = prefix.body[-1].end_col_offset
    record(prefix, prefix.name, 'rename function; retain exact original prefix through root cap; append return folded_event')
    prefix.body.append(ast.Return(value=ast.Name(id='folded_event', ctx=ast.Load())))
    selected.append(prefix)

    for name in ('memory_field', 'think'):
        method = next(n for n in runtime.body if isinstance(n, ast.FunctionDef) and n.name == name)
        record(method, name, 'bind unchanged method to minimal reference state')
        selected.append(copy.deepcopy(method))
    fixed_runtime = [n for n in tree.body if isinstance(n, ast.ClassDef) and n.name == 'SpiralcoreRuntime'][1]
    sync = next(n for n in fixed_runtime.body if isinstance(n, ast.FunctionDef) and n.name == 'sync_system_to_kuramoto')
    record(sync, sync.name, 'unchanged method; fixed_dt always provided, time module unavailable')
    selected.append(copy.deepcopy(sync))
    full = copy.deepcopy(step)
    full.name = 'reference_full_step'
    stop = next(i for i, n in enumerate(full.body) if isinstance(n, ast.Assign)
                and any(isinstance(t, ast.Name) and t.id == 'route_pkt' for t in n.targets))
    full.body = full.body[:stop]
    full.end_lineno = full.body[-1].end_lineno
    full.end_col_offset = full.body[-1].end_col_offset
    record(full, full.name, 'rename function; exact step prefix through CA invariant, before router; append return folded_event')
    full.body.append(ast.Return(value=ast.Name(id='folded_event', ctx=ast.Load())))
    selected.append(full)

    allowed = {name: getattr(__builtins__, name) if not isinstance(__builtins__, dict)
               else __builtins__[name] for name in
               ('abs', 'int', 'float', 'len', 'range', 'max', 'min', 'sum', 'zip', 'list', 'enumerate', 'round', 'ValueError')}
    ns = {'__builtins__': allowed, 'math': math, 'cmath': cmath}
    module = ast.fix_missing_locations(ast.Module(body=selected, type_ignores=[]))
    exec(compile(module, '<pinned-reference-definitions>', 'exec',
                 flags=__future__.annotations.compiler_flag, dont_inherit=True), ns)
    return ns, records


def load_reference(path):
    return compile_reference(path.read_bytes())


def fold_state(ns, cores):
    return SimpleNamespace(N=0, n_cores=cores, phases=[0., math.pi / 2, math.pi, 3 * math.pi / 2],
                           active_roots=[], valuation_3=ns['valuation_3'])


class ReferenceState(SimpleNamespace):
    @property
    def roots(self):
        return self.active_roots


def seed_phases():
    rng = random.Random(184)
    return [rng.uniform(0.0, 2.0 * math.pi) for _ in range(16)]


def runtime_state(ns, width, fixed_dt=.05, memory_enabled=True):
    phases = seed_phases()
    omegas = [.8 + (.8 * i) / 15 for i in range(16)]
    phases, order = ns['kuramoto_step'](phases, omegas, 1.75, .05)
    row = ns['reference_default_row'](width)
    _, cf = ns['encode_binary_assumption_row'](row)
    state = ReferenceState(N=0, n_cores=4, ca_width=width, n_oscillators=16,
                           phases=phases, omegas=omegas, active_roots=[],
                           valuation_3=ns['valuation_3'], system_start_time=0., last_system_time=0.,
                           fixed_dt=fixed_dt, clock=0., kuramoto_strata_time=0., sync_drift_ms=0.,
                           memory_enabled=memory_enabled, memory_gain=.35, K_coupling=.15,
                           kuramoto_K=1.75, kuramoto_R=order,
                           initial_ca184_particles=sum(row), initial_ca184_cf_particles=sum(cf))
    for name in ('memory_field', 'think', 'sync_system_to_kuramoto'):
        setattr(state, name, MethodType(ns[name], state))
    for name in ('ca184', 'ca110', 'ca30'):
        setattr(state, name + '_history', [row.copy()])
        setattr(state, name + '_cf_history', [cf.copy()])
    return state


def runtime_snapshot(state):
    return jsonable({'n': state.N, 'width': state.ca_width, 'phases': state.phases,
                     'omegas': state.omegas, 'roots': state.active_roots,
                     'rings': [getattr(state, name)[-1] for name in (
                         'ca184_history', 'ca184_cf_history', 'ca110_history',
                         'ca110_cf_history', 'ca30_history', 'ca30_cf_history')],
                     'fixed_dt': state.fixed_dt, 'memory_enabled': state.memory_enabled,
                     'memory_gain': state.memory_gain, 'k_coupling': state.K_coupling,
                     'kuramoto_k': state.kuramoto_K, 'clock': state.clock,
                     'kuramoto_order': state.kuramoto_R})


def jsonable(value):
    if isinstance(value, complex):
        return {'re': value.real, 'im': value.imag}
    if isinstance(value, dict):
        return {key: jsonable(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [jsonable(item) for item in value]
    return value


def build_vectors(ns):
    ca = []
    inputs = [('empty', [], 3), ('one_zero', [0], 8), ('one_one', [1], 8)]
    inputs += [(f'two_{pattern:02b}', [(pattern >> 1) & 1, pattern & 1], 8) for pattern in range(4)]
    inputs += [(f'neighborhood_{pattern:03b}', [pattern >> 2, (pattern >> 1) & 1, pattern & 1], 3)
               for pattern in range(8)]
    inputs += [('runtime_default_48', ns['reference_default_row'](48), 16),
               ('low_bit_mask', [-3, -2, -1, 0, 1, 2, 3], 3)]
    blocks = [('zero_256_bits', bytes(32)), ('ones_256_bits', bytes([255]) * 32),
              ('incrementing_bytes_256_bits', bytes(range(32))),
              ('alternating_bytes_256_bits', bytes([0x80, 0x01, 0xaa, 0x55]) * 8)]
    inputs += [(name, ns['_bytes_to_bits'](block), 4) for name, block in blocks]
    for number in (184, 110, 30):
        rule = ns[f'rule_{number}']
        for name, initial, count in inputs:
            states = [initial]
            for _ in range(count):
                states.append(ns['step_ca'](states[-1], rule))
            ca.append({'id': f'{number}:{name}', 'rule': number, 'input': initial,
                       'steps': count, 'states': states})
    truth = [{'rule': number, 'ascending_neighborhood_output':
              [ns[f'rule_{number}'](v >> 2, (v >> 1) & 1, v & 1) for v in range(8)]}
             for number in (184, 110, 30)]

    clock_inputs = [
        ('empty', [], [], 1.75, .05, 1),
        ('one', [0.], [1.], 1.75, .05, 8),
        ('clamp_zero_dt', [0.], [1.], 0., 0., 1),
        ('clamp_negative_dt', [.2, 1.4], [1., 1.05], 1.75, -.1, 1),
        ('clamp_tiny_dt', [.2, 1.4], [1., 1.05], 1.75, 1e-8, 1),
        ('clamp_large_dt', [.2, 1.4], [1., 1.05], 1.75, 5., 1),
        ('two_field_model', [.2, 1.4], [1., 1.05], 1.75, .05, 8),
        ('zero_order_gate', [0., math.pi], [1., 1.], 99., .05, 4),
        ('negative_phase_wrap', [-.2, 6.2], [-2., 3.], 0., .1, 4),
        ('four_heterogeneous', [.1, .7, 2.4, 5.9], [.8, 1., 1.2, 1.6], 1.75, .05, 8),
        ('sixteen_explicit', [i * .37 for i in range(16)], [.8 + .8 * i / 15 for i in range(16)], 1.75, .05, 4),
    ]
    clocks = []
    for name, phases, omegas, coupling, dt, count in clock_inputs:
        current, outputs = phases, []
        for _ in range(count):
            current, order = ns['kuramoto_step'](current, omegas, coupling, dt)
            outputs.append({'phases': current, 'order': order})
        clocks.append({'id': name, 'phases': phases, 'omegas': omegas, 'K': coupling,
                       'dt': dt, 'steps': count, 'outputs': outputs})
    mismatches = []
    for name, phases, omegas in [('short', [0., 0.], [0.]), ('missing', [.2], []), ('excess', [.2], [1., 2.])]:
        out, order = ns['kuramoto_step'](phases, omegas, 0., .05)
        mismatches.append({'id': name, 'phases': phases, 'omegas': omegas, 'K': 0., 'dt': .05,
                           'observed_output': out, 'observed_order': order,
                           'classification': 'historical permissive input; proposed Rust rejects mismatched lengths'})
    n2 = []
    for args in [(0.2, 1.4, 1., 1.05, 1.75, .05), (0., 0., 1., 1., 1.75, .05),
                 (6.2, .1, 1., 1., 1.75, .05), (-.2, 7., -1., 2., 0., .1)]:
        n2.append({'input_theta1_theta2_omega1_omega2_K_dt': args,
                   'output_theta1_theta2_delta': ns['kuramoto_n2_field_step'](*args)})

    valuation = []
    for n in [0, 1, 2, 3, 6, 9, 27, 81, 243, -1, -3, -81, 3 ** 20, 3 ** 20 - 1]:
        try:
            valuation.append({'input': n, 'output': ns['valuation_3'](n)})
        except ValueError as exc:
            valuation.append({'input': n, 'error': str(exc)})
    carry = []
    for name, values in [('ascending_81', [float(i) for i in range(1, 82)]),
                         ('alternating_27', [float((i % 2) * 2 - 1) for i in range(27)])]:
        state, outputs = fold_state(ns, 4), []
        for value in values:
            event = ns['reference_fold_prefix'](state, value, '')
            outputs.append(jsonable({'N': state.N, 'event': event, 'roots': copy.deepcopy(state.active_roots)}))
        carry.append({'id': name, 'n_cores': 4, 'constant_phases': state.phases, 'input_values': values,
                      'outputs': outputs})
    runtime_cases = []
    for name, width, count, dt, memory in [('default_48_steps81', 48, 81, .05, True),
                                         ('one_cell', 1, 9, .05, True), ('two_cells', 2, 9, .05, True),
                                         ('bits256', 256, 9, .05, True),
                                         ('memory_disabled', 48, 9, .05, False),
                                         ('dt_low_clamp', 48, 3, .001, True),
                                         ('dt_high_clamp', 48, 3, .5, True)]:
        state = runtime_state(ns, width, dt, memory)
        snapshots = [runtime_snapshot(state)]
        for _ in range(count):
            ns['reference_full_step'](state, 1., '')
            snapshots.append(runtime_snapshot(state))
        runtime_cases.append({'id': name, 'width': width, 'steps': count, 'payload': 1.,
                              'fixed_dt': dt, 'memory_enabled': memory, 'snapshots': snapshots})
    raw_phases = seed_phases()
    return {'schema': 'spiralcore.python-reference.v1', 'source_sha256': SOURCE_SHA256,
            'ca_truth_tables': truth, 'ca': ca,
            'bit_order_examples': [{'bytes_hex': b.hex(), 'bits': ns['_bytes_to_bits'](b)} for _, b in blocks],
            'kuramoto': clocks, 'kuramoto_input_mismatch_observations': mismatches,
            'kuramoto_n2_field': n2, 'valuation_3': valuation, 'runtime_fold_prefix': carry,
            'initial_phase_seed': {'method': 'stdlib random.Random(184).uniform(0,2*pi)',
                                   'raw_phases': raw_phases,
                                   'raw_f64_bits_be_hex': [struct.pack('>d', x).hex() for x in raw_phases]},
            'runtime_transition': runtime_cases}


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', type=pathlib.Path, required=True)
    parser.add_argument('--wrapper', type=pathlib.Path, required=True)
    parser.add_argument('--extraction', type=pathlib.Path, required=True)
    parser.add_argument('--out', type=pathlib.Path, required=True)
    args = parser.parse_args()
    if sha(args.wrapper.read_bytes()) != WRAPPER_SHA256:
        raise ValueError('wrapper SHA-256 does not match pinned original')
    extraction_bytes = args.extraction.read_bytes()
    extraction = json.loads(extraction_bytes)
    assert extraction['source_sha256'] == WRAPPER_SHA256
    assert next(m for m in extraction['modules'] if m['module'] == 'spiralcore_kernel')['sha256'] == SOURCE_SHA256
    ns, provenance = load_reference(args.source)
    args.out.mkdir(parents=True, exist_ok=True)
    vectors = build_vectors(ns)
    write_json(args.out / 'vectors.json', vectors)
    write_json(args.out / 'source-excerpts.json', provenance)
    (args.out / 'extraction.json').write_bytes(extraction_bytes)
    write_json(args.out / 'generation.json', {
        'schema': 'spiralcore.reference-generation.v1', 'argv': sys.argv, 'exit_code': 0,
        'python': sys.version, 'platform': platform.platform(), 'machine': platform.machine(),
        'generator_sha256': sha(pathlib.Path(__file__).read_bytes()),
        'source': str(args.source), 'source_sha256': SOURCE_SHA256,
        'wrapper': str(args.wrapper), 'wrapper_sha256': WRAPPER_SHA256,
        'original_extraction_sha256': sha(extraction_bytes),
        'execution_scope': list(EXPORTED),
        'excluded_execution': ['module imports/startup', 'runtime initialization', 'cards', 'UI', 'autosave', 'wall clock', 'network'],
        'random_scope': 'Explicit deterministic stdlib Random(184) phase draws only; no runtime initialization.',
        'scope_note': 'Definitions compiled from verified AST; runtime transition stops before router/audio/audit/card paths. Logical fixed time starts at0.',
        'numeric_note': 'CA/integer fixtures require exact equality. Floating values record Python/libm outputs; cross-language comparison uses declared tolerance, not bit identity.',
        'source_files_unchanged_after': sha(args.source.read_bytes()) == SOURCE_SHA256 and sha(args.wrapper.read_bytes()) == WRAPPER_SHA256,
    })
    print(json.dumps({'status': 'generated', 'ca_cases': len(vectors['ca']),
                      'kuramoto_cases': len(vectors['kuramoto']), 'fold_sequences': len(vectors['runtime_fold_prefix']),
                      'runtime_cases': len(vectors['runtime_transition']),
                      'runtime_transitions': sum(c['steps'] for c in vectors['runtime_transition'])}))


if __name__ == '__main__':
    main()
