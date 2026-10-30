"""Qualification of bounded historical-reference extraction, not production tests."""
import pathlib
import unittest

import generate_reference as reference


SOURCE = pathlib.Path('/Users/casey.allard/spiralcore/local/shared-state-20261030/reference/spiralcore_kernel.py')


class ReferenceExtractionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.ns, cls.provenance = reference.load_reference(SOURCE)

    def test_rejects_unpinned_source_before_execution(self):
        with self.assertRaisesRegex(ValueError, 'source SHA-256'):
            reference.compile_reference(b'raise RuntimeError("must not execute")')

    def test_rules_match_independent_elementary_truth_table(self):
        for number in (184, 110, 30):
            fn = self.ns[f'rule_{number}']
            for pattern in range(8):
                self.assertEqual(fn(pattern >> 2, (pattern >> 1) & 1, pattern & 1), (number >> pattern) & 1)

    def test_empty_and_small_periodic_rings_and_mask(self):
        step, rule = self.ns['step_ca'], self.ns['rule_184']
        self.assertEqual(step([], rule), [])
        self.assertEqual(step([1], rule), [1])
        self.assertEqual(step([1, 0], rule), [0, 1])
        self.assertEqual(step([3, 2], rule), [0, 1])

    def test_default_row_and_msb_byte_order(self):
        row = self.ns['reference_default_row'](48)
        self.assertEqual(len(row), 48)
        self.assertEqual(sum(row), 21)
        self.assertEqual(self.ns['_bytes_to_bits'](b'\x80\x01'), [1] + [0] * 14 + [1])

    def test_kuramoto_clamp_and_historical_length_mismatch(self):
        step = self.ns['kuramoto_step']
        self.assertEqual(step([], [], 1.75, .05), ([], 0.0))
        self.assertEqual(step([0.0], [1.0], 0., 0.), ([.0001], 1.0))
        self.assertEqual(step([0.0], [1.0], 0., 5.), ([.5], 1.0))
        phases, order = step([0., 0.], [0.], 0., .05)
        self.assertEqual(phases, [0.])
        self.assertEqual(order, .5)

    def test_observed_fold_is_unweighted_lossy_summary(self):
        state = reference.fold_state(self.ns, 1)
        for value in range(1, 10):
            self.ns['reference_fold_prefix'](state, float(value), '')
        self.assertEqual(state.N, 9)
        self.assertEqual(len(state.active_roots), 1)
        self.assertEqual(state.active_roots[0]['memory_count'], 9)
        self.assertEqual(state.active_roots[0]['value'], 6.2)
        self.assertNotEqual(state.active_roots[0]['value'], sum(range(1, 10)) / 9)

    def test_scope_excludes_runtime_io_and_initialization(self):
        self.assertEqual(set(self.ns) - {'__builtins__', 'math', 'cmath'}, set(reference.EXPORTED))
        self.assertTrue(all(item['source_start_line'] <= item['source_end_line'] for item in self.provenance))

    def test_full_transition_uses_fixed_time_and_preserves_particles(self):
        state = reference.runtime_state(self.ns, 48)
        for _ in range(9):
            self.ns['reference_full_step'](state, 1.0, '')
        self.assertEqual(state.N, 9)
        self.assertEqual(state.last_system_time, .45)
        self.assertEqual(state.active_roots[0]['memory_count'], 9)
        self.assertEqual(sum(state.ca184_history[-1]), 21)
        self.assertEqual(sum(state.ca184_cf_history[-1]), 27)
        self.assertEqual(len(state.phases), 16)


if __name__ == '__main__':
    unittest.main(verbosity=2)
