#!/usr/bin/env python3
"""Read-only SC-OPT-003 evidence analysis; writes a new review receipt only.

Run from this file's directory with Python 3. Output is exclusive-create, so an
existing review is never overwritten. No binaries, builds, tests, or measurements
are executed. Statistics use all eight matched pairs and exact integer nanoseconds.
"""
import collections
import datetime
import hashlib
import json
import math
import pathlib
import re
import statistics

ROOT = pathlib.Path(__file__).resolve().parent
REPO = pathlib.Path('/Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore')
PAIRED = ROOT / 'paired'
REVISION = '80449939bd92348593cc88555cf7fde0847ceffd'
BASE = 'c7fceaf53550e0ab1393604c2bc88eabd267676766ae90ad52459ece381822c0'
TARGET = '227bbf10e522740c837d03da96838331a00fe3dac2ca77c095ba1044467b0ebf'
NAMESPACE = 'c017eb0b2e6c1cae26a0f1818b7089a486ebe712ace4cd79259a2521045c039c'
POPULATIONS = [0, 64, 256, 768]
ARMS = ['raw-node-budget-262144', 'sparse-node-budget-262144',
        'raw-node-budget-1', 'sparse-node-budget-1']

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def load(path):
    return json.loads(path.read_text())

def lines(path):
    return [json.loads(line) for line in path.read_text().splitlines()]

def manifest(path, root):
    count = 0
    for line in path.read_text().splitlines():
        expected, name = line.split('  ', 1)
        target = pathlib.Path(name)
        if not target.is_absolute():
            target = root / target
        assert digest(target) == expected, str(target)
        count += 1
    return count

def get(row, dotted):
    for part in dotted.split('.'):
        row = row[part]
    return row

def statistics_for(software, hardware):
    assert len(software) == len(hardware) == 8
    ratios = [s / h for s, h in zip(software, hardware)]
    return {
        'software_ns': software, 'hardware_ns': hardware,
        'software_over_hardware_ratios': ratios,
        'paired_geometric_mean': math.exp(sum(math.log(x) for x in ratios) / 8),
        'hardware_faster_pairs': sum(s > h for s, h in zip(software, hardware)),
        'ties': sum(s == h for s, h in zip(software, hardware)),
        'software_midpoint_median_ns': statistics.median(software),
        'hardware_midpoint_median_ns': statistics.median(hardware),
        'software_nearest_rank_p95_ns': max(software),
        'hardware_nearest_rank_p95_ns': max(hardware),
    }

qualification = load(ROOT / 'qualification.json')
assert qualification['source_revision'] == REVISION
schedule = load(ROOT / 'schedule.json')
assert digest(ROOT / 'schedule.json') == qualification['schedule_sha256']
assert manifest(PAIRED / 'driver-inputs.sha256', ROOT) == 4
assert manifest(PAIRED / 'observations.sha256', ROOT) == 16
assert (PAIRED / 'templates-before.sha256').read_bytes() == (PAIRED / 'templates-after.sha256').read_bytes()
assert (PAIRED / 'templates-before.sha256').read_bytes() == (ROOT / 'templates-established.sha256').read_bytes()
template_root = pathlib.Path(schedule['templates'])
template_entries = manifest(PAIRED / 'templates-after.sha256', template_root)

for variant in ('software', 'hardware'):
    build = ROOT / ('build-' + variant)
    assert manifest(build / 'executables.sha256', ROOT) == 2
    assert manifest(build / 'helper-files.sha256', ROOT) == 13
    manifest(build / 'production-source-files.sha256', REPO)
    assert digest(build / 'production-source-files.sha256') == qualification['source_fingerprint_sha256']
    assert digest(build / 'receiver-probe') == qualification['configurations'][variant]['receiver_probe_sha256']

order = [(p['block'], v) for p in schedule['pairs'] for v in p['order']]
assert len(order) == 16
assert [(int(b), v) for b, v in (line.split() for line in (PAIRED / 'execution-order.txt').read_text().splitlines())] == order
events = lines(PAIRED / 'execution.jsonl')
assert len(events) == 32
previous_end = None
for i, (block, variant) in enumerate(order):
    start, end = events[2 * i:2 * i + 2]
    alias = 'old' if variant == 'software' else 'new'
    assert start['event'] == 'start' and end['event'] == 'end' and end['exit_code'] == 0
    for event in (start, end):
        assert (event['block'], event['configuration'], event['receipt_variant']) == (block, variant, alias)
    assert start['utc'] <= end['utc']
    assert previous_end is None or previous_end <= start['utc']
    previous_end = end['utc']

# Independently regenerate this diagnostic's per-byte xorshift fixture. It is not
# the earlier WAN generator, which sampled eight bytes from each state.
state = 42
base = bytearray()
mask = (1 << 64) - 1
for _ in range(1048576):
    state ^= (state << 13) & mask
    state ^= state >> 7
    state ^= (state << 17) & mask
    base.append(state & 255)
target = bytearray(base)
for i in range(524288, 524320):
    target[i] ^= 165
def object_id(data):
    return hashlib.sha256(b'spiralcore.object.bytes.v1\0' + len(data).to_bytes(8, 'little') + data).hexdigest()
assert object_id(base) == BASE and object_id(target) == TARGET

rows = {}
work_paths = set()
application_bytes = collections.Counter()
frames_total = 0
all_metric_names = [f'{field}.{dimension}'
                    for field in ('clone_and_sync', 'endpoint_open', 'frame_preparation',
                                  'handler_stages.begin', 'handler_stages.data',
                                  'handler_stages.finish', 'handler_stages.total',
                                  'dispatch_including_instrumentation', 'local_operation_total',
                                  'close_reopen_verify')
                    for dimension in ('cpu_ns', 'wall_ns')]
minimum_operation_residual = {'cpu_ns': None, 'wall_ns': None}
for block, variant in order:
    file = PAIRED / f'block{block}-{variant}.jsonl'
    contents = lines(file)
    assert len(contents) == 17
    metadata, observations = contents[0], contents[1:]
    alias = 'old' if variant == 'software' else 'new'
    assert metadata['kind'] == 'metadata'
    assert metadata['library_revision'] == REVISION
    assert metadata['library_source_sha256'] == qualification['source_fingerprint_sha256']
    assert metadata['executable_sha256'] == qualification['configurations'][variant]['receiver_probe_sha256']
    assert (metadata['block'], metadata['variant']) == (block, alias)
    assert metadata['fixture_generator'] == 'xorshift64-low-byte-per-step-v1'
    assert metadata['fixture_seed'] == 42 and metadata['distinct_from_wan_fixture']
    assert metadata['policies'] == {'normal_nodes': 262144, 'tiny_nodes': 1,
                                   'graph_cache_entries': 0, 'graph_cache_bytes': 0}
    expected_order = []
    for population_index in range(4):
        n = POPULATIONS[(population_index + block) % 4]
        offset = (block + POPULATIONS.index(n)) % 4
        expected_order.extend((n, ARMS[(j + offset) % 4]) for j in range(4))
    assert [(r['population'], r['arm_label']) for r in observations] == expected_order
    for row in observations:
        n, arm = row['population'], row['arm_label']
        sparse, tiny = arm.startswith('sparse'), arm.endswith('-1')
        assert row['kind'] == 'observation' and row['variant'] == alias and row['block'] == block
        assert row['arm'] == {'sparse': sparse, 'tiny': tiny}
        assert row['status'] == 'exact' and row['error'] is None and row['error_stage'] == ''
        assert row['base_object'] == BASE and row['target_object'] == TARGET
        assert row['object_bytes'] == 1048576 and row['active_namespace'] == NAMESPACE
        assert row['work_cache'] not in work_paths and not pathlib.Path(row['work_cache']).exists()
        work_paths.add(row['work_cache'])
        ready = not sparse and not tiny
        before = {'namespaces': n // 2 + 1, 'publications': n + 1,
                  'publication_bytes': 1048657 + 113 * n, 'active_publications': 1,
                  'active_bytes': 1048657, 'manifests': n + 1}
        target_publication_size = 1048576 + (81 if ready else 49)
        after = {**before, 'publications': n + 2,
                 'publication_bytes': before['publication_bytes'] + target_publication_size,
                 'active_publications': 2, 'active_bytes': 1048657 + target_publication_size,
                 'manifests': n + 1 + int(ready)}
        assert row['before'] == before and row['after'] == after
        assert row['target_manifest_present'] == ready
        assert row['graph_cache_after'] == {'entries': 0, 'evictions': 0, 'hits': 0,
                                           'misses': 0, 'resident_bytes': 0}
        samples = row['frame_samples']
        requests = [134, 128, 56] if sparse else [102] + [61501] * 17 + [4157, 56]
        replies = [66] + [64] * (len(requests) - 2) + [88]
        assert len(samples) == len(requests)
        for i, frame in enumerate(samples):
            kind = 1 if i == 0 else 3 if i == len(samples) - 1 else 2
            assert frame['sequence'] == i and frame['kind'] == kind
            assert frame['request_bytes'] == requests[i]
            assert frame['reply_kind'] == kind + 100 and frame['reply_bytes'] == replies[i]
            assert not frame['reply_body_truncated']
            if kind == 1:
                expected_body = bytes([3, 1] if sparse else [0, 0]) + (1).to_bytes(8, 'little')
            elif kind == 2:
                accepted = 1048576 if sparse else min(i * 61440, 1048576)
                expected_body = accepted.to_bytes(8, 'little')
            else:
                expected_body = bytes.fromhex(TARGET)
            assert bytes(frame['reply_body_prefix']) == expected_body
            assert frame['cost']['frames'] == 1
            assert all(isinstance(frame['cost'][x], int) and frame['cost'][x] >= 0
                       for x in ('cpu_ns', 'wall_ns'))
        for stage, kind in (('begin', 1), ('data', 2), ('finish', 3)):
            for unit in ('cpu_ns', 'wall_ns', 'frames'):
                assert row['handler_stages'][stage][unit] == sum(
                    f['cost'][unit] for f in samples if f['kind'] == kind)
        for unit in ('cpu_ns', 'wall_ns', 'frames'):
            assert row['handler_stages']['total'][unit] == sum(f['cost'][unit] for f in samples)
        for unit in ('cpu_ns', 'wall_ns'):
            assert row['dispatch_including_instrumentation'][unit] >= row['handler_stages']['total'][unit]
            residual = row['local_operation_total'][unit] - row['dispatch_including_instrumentation'][unit] - row['frame_preparation'][unit]
            assert residual >= 0
            previous = minimum_operation_residual[unit]
            minimum_operation_residual[unit] = residual if previous is None else min(previous, residual)
        for field in ('dispatch_including_instrumentation', 'local_operation_total'):
            assert row[field]['frames'] == len(samples)
        for field in ('clone_and_sync', 'endpoint_open', 'frame_preparation', 'close_reopen_verify'):
            assert row[field]['frames'] == 0
        assert all(isinstance(get(row, metric), int) and get(row, metric) > 0 for metric in all_metric_names)
        size = sum(requests) + sum(replies)
        assert size == (536 if sparse else 1051138)
        application_bytes[variant] += size
        frames_total += len(samples)
        key = (n, arm, block, variant)
        assert key not in rows
        rows[key] = row
assert len(rows) == len(work_paths) == 256
assert frames_total == 2944
assert application_bytes['software'] == application_bytes['hardware']

cells = []
for n in POPULATIONS:
    for arm in ARMS:
        metrics = {}
        for metric in all_metric_names:
            software = [get(rows[n, arm, b, 'software'], metric) for b in range(8)]
            hardware = [get(rows[n, arm, b, 'hardware'], metric) for b in range(8)]
            metrics[metric] = statistics_for(software, hardware)
        cells.append({'population': n, 'arm': arm, 'pairs': 8,
                      'modeled_bidirectional_application_bytes_per_observation': 536 if arm.startswith('sparse') else 1051138,
                      'metrics': metrics})
gates = []
for expected in schedule['joint_target']['cells']:
    cell = next(c for c in cells if (c['population'], c['arm']) == (expected['population'], expected['arm']))
    metric = cell['metrics']['local_operation_total.cpu_ns']
    threshold = expected['minimum_paired_software_over_hardware_geometric_mean']
    gates.append({**expected, 'observed_geometric_mean': metric['paired_geometric_mean'],
                  'hardware_faster_pairs': metric['hardware_faster_pairs'],
                  'passed': metric['paired_geometric_mean'] >= threshold and metric['hardware_faster_pairs'] >= 6})
loads = []
for path in PAIRED.glob('*-host-before.txt'):
    match = re.search(r'load averages?:\s*([\d.]+)', path.read_text())
    assert match, str(path)
    loads.append(float(match.group(1)))
assert len(loads) == 16

report = {
    'schema': 'spiralcore.sha-independent-measurement-review.v1',
    'reviewer': 'transport_audit', 'status': 'passed',
    'reviewed_at_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'experiment': 'SC-OPT-003', 'source_revision': REVISION,
    'qualification_sha256': digest(ROOT / 'qualification.json'),
    'source_fingerprint_sha256': qualification['source_fingerprint_sha256'],
    'schedule_sha256': digest(ROOT / 'schedule.json'),
    'analysis_procedure_sha256': digest(pathlib.Path(__file__).resolve()),
    'observations_manifest_sha256': digest(PAIRED / 'observations.sha256'),
    'execution_sha256': digest(PAIRED / 'execution.jsonl'),
    'templates_manifest_sha256': digest(PAIRED / 'templates-before.sha256'),
    'binary_sha256': {v: qualification['configurations'][v]['receiver_probe_sha256'] for v in ('software', 'hardware')},
    'checks': {'processes': 16, 'exit_zero': 16, 'exact_observations': 256,
               'cells': 16, 'pairs_per_cell': 8, 'filtered_rows': 0, 'retries': 0,
               'fixed_external_and_internal_order_match': True,
               'metadata_source_and_executable_bindings_match': True,
               'template_file_entries_checked': template_entries,
               'templates_equal_before_after_and_original': True,
               'fixtures_independently_regenerated': True,
               'all_ids_inventory_readiness_and_retention_match': True,
               'all_frame_sequences_lengths_reply_bodies_and_stage_sums_match': True,
               'minimum_operation_minus_preparation_minus_dispatch_ns': minimum_operation_residual,
               'all_successful_work_caches_absent_after_cleanup': True,
               'frame_samples_checked': frames_total,
               'modeled_application_frame_bytes': dict(application_bytes),
               'reopened_exact_target_bytes_reported': 256 * 1048576,
               'reopened_exact_base_bytes_reported': 256 * 1048576},
    'statistics': {'ratio': 'software/hardware; greater than one favors hardware',
                   'paired_geometric_mean': 'exp(sum(log(software_i/hardware_i))/8)',
                   'marginal_median': 'midpoint of fourth and fifth sorted observations',
                   'p95': 'nearest rank ceil(0.95*8); maximum of eight, not a stable tail estimate',
                   'filtering_or_control_normalization': False},
    'primary_joint_gate_passed': all(g['passed'] for g in gates), 'registered_gates': gates,
    'host_one_minute_load_range': [min(loads), max(loads)], 'cells': cells,
    'limitations': ['This is a same-source native Apple ARM local process experiment, not WAN or energy measurement.',
                    'Primary process CPU includes preparation, endpoint handling and request destruction; handler CPU is a subset and must not be added.',
                    'Clone/sync, endpoint open, fixture generation and exact reopened verification are outside the primary operation and separately recorded.',
                    'OS page caches and host scheduling/load are uncontrolled; no post-hoc normalization or discarded observations.',
                    'Raw normal builds a target DAG; sparse and tiny policies have different target readiness. Only identical off/on arms are compared.',
                    'Repeated blocks use the same fixed fixture, not a workload distribution or unsupported-platform qualification.',
                    'Reopened byte comparisons were performed by the frozen probe. The reviewer validates receipts and identities, not deleted working caches.',
                    'No tests, builds, binaries or further measurements were executed by the reviewer.'],
}
destination = ROOT / 'independent-measurement-review.json'
with destination.open('x') as out:
    json.dump(report, out, indent=2)
    out.write('\n')
print(json.dumps({'status': report['status'], 'observations': len(rows),
                  'joint_gate_passed': report['primary_joint_gate_passed'],
                  'gates': gates, 'review_sha256': digest(destination)}))
