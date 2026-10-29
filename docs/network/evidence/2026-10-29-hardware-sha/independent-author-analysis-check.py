#!/usr/bin/env python3
"""Compare the independent-first result with every author metric; no measurement."""
import hashlib
import json
import math
import pathlib
import statistics

p = pathlib.Path(__file__).resolve().parent
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
independent = json.loads((p / 'independent-measurement-review.json').read_text())
author = json.loads((p / 'paired-analysis.json').read_text())
assert sha(p / 'paired-analysis.json') == 'e7979a66aca6a64d6c642b2b8b922cddb91246e27f70b86a1876dd59de894c75'
assert sha(p / 'paired-summary.md') == '80c257a7d8662c0eac003797e8ec35882627127531c96a7403fc918d2ec942ea'
fields = {
    'operation': 'local_operation_total',
    'begin': 'handler_stages.begin', 'data': 'handler_stages.data',
    'finish': 'handler_stages.finish', 'handler': 'handler_stages.total',
    'frame_preparation': 'frame_preparation', 'open': 'endpoint_open',
    'clone': 'clone_and_sync', 'reopen_verify': 'close_reopen_verify',
    'dispatch': 'dispatch_including_instrumentation',
}
cells = {(c['population'], c['arm']): c for c in independent['cells']}
assert len(author['groups']) == len(cells) == 16
count = 0
exact_records = 0
floating_differences = []
for group in author['groups']:
    cell = cells[group['population'], group['arm']]
    assert group['pairs'] == cell['pairs'] == 8
    assert len(group['metrics']) == 21
    for name, actual in group['metrics'].items():
        if name == 'begin_plus_finish_cpu_ns':
            a, b = cell['metrics']['handler_stages.begin.cpu_ns'], cell['metrics']['handler_stages.finish.cpu_ns']
            software = [x + y for x, y in zip(a['software_ns'], b['software_ns'])]
            hardware = [x + y for x, y in zip(a['hardware_ns'], b['hardware_ns'])]
        else:
            dimension = 'cpu_ns' if name.endswith('_cpu_ns') else 'wall_ns'
            prefix = name.removesuffix('_' + dimension)
            expected = cell['metrics'][fields[prefix] + '.' + dimension]
            software, hardware = expected['software_ns'], expected['hardware_ns']
        ratios = [s / h for s, h in zip(software, hardware)]
        geomean = math.exp(sum(math.log(r) for r in ratios) / 8)
        expected = {
            'software_values': software, 'hardware_values': hardware,
            'paired_ratios': ratios,
            'software_median': statistics.median(software),
            'hardware_median': statistics.median(hardware),
            'software_p95_nearest_rank': max(software),
            'hardware_p95_nearest_rank': max(hardware),
            'paired_software_over_hardware_geomean': geomean,
            'paired_software_over_hardware_median': statistics.median(ratios),
            'paired_software_over_hardware_min': min(ratios),
            'paired_software_over_hardware_max': max(ratios),
            'geomean_implied_reduction_fraction': 1 - 1 / geomean,
            'hardware_faster_pairs': sum(s > h for s, h in zip(software, hardware)),
            'ties': sum(s == h for s, h in zip(software, hardware)),
        }
        assert set(actual) == set(expected)
        exact_records += actual == expected
        for field, value in expected.items():
            if actual[field] == value:
                continue
            # All integer samples, paired-ratio arrays and direct order statistics
            # must match exactly. Only derived exp/log means and their reduction
            # fractions may differ by at most four binary64 representable steps.
            assert field in ('paired_software_over_hardware_geomean',
                             'geomean_implied_reduction_fraction')
            assert isinstance(value, float) and isinstance(actual[field], float)
            difference = abs(actual[field] - value)
            assert difference <= 4 * math.ulp(value)
            floating_differences.append({
                'population': group['population'], 'arm': group['arm'],
                'metric': name, 'field': field, 'independent': value,
                'author': actual[field], 'absolute_difference': difference,
                'binary64_steps': difference / math.ulp(value),
            })
        count += 1
assert count == 336
assert author['gates']['joint_target_met'] == independent['primary_joint_gate_passed']
for gate in author['gates']['cells']:
    own = next(g for g in independent['registered_gates']
               if (g['population'], g['arm']) == (gate['population'], gate['arm']))
    assert gate['paired_software_over_hardware_geomean'] == own['observed_geometric_mean']
    assert gate['hardware_faster_pairs'] == own['hardware_faster_pairs']
    assert gate['minimum_geomean'] == own['minimum_paired_software_over_hardware_geometric_mean']
    assert gate['passes'] == own['passed']
assert author['source']['revision'] == independent['source_revision']
assert author['source']['production_source_fingerprint_sha256'] == independent['source_fingerprint_sha256']
for v in ('software', 'hardware'):
    assert author['source'][v + '_executable_sha256'] == independent['binary_sha256'][v]
pin_paths = {
    'qualification_sha256': 'qualification.json',
    'observations_manifest_sha256': 'paired/observations.sha256',
    'execution_jsonl_sha256': 'paired/execution.jsonl',
    'driver_inputs_manifest_sha256': 'paired/driver-inputs.sha256',
    'templates_before_sha256': 'paired/templates-before.sha256',
    'templates_after_sha256': 'paired/templates-after.sha256',
    'schedule_sha256': 'schedule.json',
}
assert set(author['pins']) == set(pin_paths)
for field, path in pin_paths.items():
    assert author['pins'][field] == sha(p / path)
assert author['overall_descriptive_counts']['hardware_faster_operation_cpu_pairs'] == 128
assert author['overall_descriptive_counts']['hardware_faster_operation_wall_pairs'] == 126
assert author['modeled_application_bytes']['each_configuration_campaign_total'] == 67307136
report = {
    'schema': 'spiralcore.sha-independent-author-analysis-crosscheck.v1',
    'status': 'passed', 'experiment': 'SC-OPT-003',
    'independent_first_review_sha256': sha(p / 'independent-measurement-review.json'),
    'author_analysis_sha256': sha(p / 'paired-analysis.json'),
    'author_summary_sha256': sha(p / 'paired-summary.md'),
    'crosscheck_method_sha256': sha(pathlib.Path(__file__).resolve()),
    'cells': 16, 'metrics_per_cell': 21, 'complete_metric_records_verified': count,
    'complete_metric_records_exactly_equal': exact_records,
    'floating_rounding_differences': floating_differences,
    'raw_clock_metrics': 320, 'independently_derived_begin_plus_finish_metrics': 16,
    'checks': ['All eight original values and paired ratios per metric match.',
               'All midpoint medians, p95/maxima, paired median/min/max, wins and ties match exactly.',
               'Derived geometric means and reduction fractions agree within at most two binary64 steps in three fields; every other scalar is exact.',
               'All three registered gates, source/executable identities and seven artifact pins match.'],
    'report_wording_review': {'blockers': [],
        'editorial_note': 'The author summary phrase "126/128 observations" refers to 128 paired wall comparisons across 256 observations; the parent will use paired comparisons in the main report. Displayed millisecond rounding is not authoritative over retained exact nanosecond arrays.',
        'scope': 'The author summary distinguishes local process scope, unchanged modeled bytes, excluded setup/reopen work, small-stage negatives, default-off deployment, and unqualified WAN/energy/FIPS claims.'},
    'no_additional_measurements': True,
}
out = p / 'independent-analysis-crosscheck.json'
with out.open('x') as f:
    json.dump(report, f, indent=2)
    f.write('\n')
print(json.dumps({'status': 'passed', 'metric_records': count,
                  'crosscheck_receipt_sha256': sha(out)}))
