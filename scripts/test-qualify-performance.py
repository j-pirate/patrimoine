#!/usr/bin/env python3
"""Exercise the supervisor with real subprocess failures, without a user vault."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name('qualify-performance.py')


class SupervisorTests(unittest.TestCase):
    def run_driver(self, scenario, session):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            output = root / 'report'
            (output / 'vault').mkdir(parents=True)
            (output / 'vault/expected.json').write_text('{"profile":"pilot"}')
            driver = root / 'driver'
            driver.write_text(f'''#!{sys.executable}
import json, os, signal, sys, time
scenario = {scenario!r}
if scenario == 'timeout':
    time.sleep(30)
if scenario == 'invalid':
    print('not JSON', flush=True)
else:
    print(json.dumps({{'operation':'open', 'status':'passed', 'open_seconds':0.01, 'exit_code':0}}), flush=True)
if scenario == 'signal':
    os.kill(os.getpid(), signal.SIGTERM)
if scenario == 'exit7':
    sys.exit(7)
if scenario == 'shutdown_timeout':
    time.sleep(30)
if '--session' in sys.argv:
    for line in sys.stdin:
        pass
''')
            driver.chmod(0o700)
            command = [sys.executable, str(SCRIPT), '--reuse', '--output', str(output),
                       '--binary', str(driver), '--operations', 'open', '--timeout', '0.2']
            if session:
                command.append('--session')
            process = subprocess.run(command, capture_output=True, text=True, timeout=10)
            report = json.loads((output / 'performance.json').read_text())
            return process.returncode, report

    def test_success_requires_normal_exit_in_both_modes(self):
        for session in (False, True):
            with self.subTest(session=session):
                code, report = self.run_driver('success', session)
                self.assertEqual(code, 0)
                record = report['measurements'][0]
                self.assertEqual(record['status'], 'passed')
                self.assertEqual(record['exit_code'], 0)
                self.assertEqual(record['process_state'], 'exited')
                self.assertTrue(record['provisional_target_met'])

    def test_json_success_does_not_mask_nonzero_exit_or_signal(self):
        for session in (False, True):
            for scenario, expected in [('exit7', 7), ('signal', -15)]:
                with self.subTest(session=session, scenario=scenario):
                    code, report = self.run_driver(scenario, session)
                    self.assertEqual(code, 1)
                    record = report['measurements'][0]
                    self.assertEqual(record['driver_status'], 'passed')
                    self.assertEqual(record['status'], 'error')
                    self.assertEqual(record['exit_code'], expected)
                    self.assertFalse(record['provisional_target_met'])

    def test_invalid_json_and_timeout_fail_in_both_modes(self):
        for session in (False, True):
            for scenario in ('invalid', 'timeout'):
                with self.subTest(session=session, scenario=scenario):
                    code, report = self.run_driver(scenario, session)
                    self.assertEqual(code, 1)
                    record = report['measurements'][0]
                    self.assertNotEqual(record['status'], 'passed')
                    self.assertFalse(record['provisional_target_met'])
                    if scenario == 'invalid':
                        self.assertTrue(record['invalid_stdout'])
                    else:
                        self.assertEqual(record['status'], 'timeout')
                        self.assertLess(record['exit_code'], 0)

    def test_session_json_is_pending_until_process_shutdown_finishes(self):
        code, report = self.run_driver('shutdown_timeout', True)
        self.assertEqual(code, 1)
        record = report['measurements'][0]
        self.assertEqual(record['driver_status'], 'passed')
        self.assertEqual(record['status'], 'timeout')
        self.assertTrue(record['shutdown_timeout'])
        self.assertFalse(record['provisional_target_met'])


if __name__ == '__main__':
    unittest.main()
