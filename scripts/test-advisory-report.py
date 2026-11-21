#!/usr/bin/env python3
import importlib.util
from datetime import date
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('advisory_report', Path(__file__).with_name('check-advisory-report.py'))
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class ReviewTests(unittest.TestCase):
    def test_known_warning_stays_visible_and_new_version_requires_review(self):
        entry = {'id': 'RUSTSEC-fixture', 'package': 'fixture', 'version': '1.0', 'kind': 'unsound'}
        policy = {'tracked_warnings': [entry], 'review_again_before': '2026-12-12'}
        warning = {'advisory': {'id': entry['id']}, 'package': {'name': 'fixture', 'version': '1.0'}}
        report = {'warnings': {'unsound': [warning]}}
        warnings, unexpected, expired = checker.review(report, policy, date(2026, 11, 20))
        self.assertEqual(len(warnings), 1)
        self.assertEqual(unexpected, [])
        self.assertFalse(expired)
        warning['package']['version'] = '1.1'
        self.assertEqual(len(checker.review(report, policy, date(2026, 11, 20))[1]), 1)
        self.assertTrue(checker.review(report, policy, date(2026, 12, 12))[2])


if __name__ == '__main__':
    unittest.main()
