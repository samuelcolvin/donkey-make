import os
import subprocess
import sys
from pathlib import Path

import pytest

THIS_DIR = Path(__file__).parent


@pytest.mark.skipif('--cov' not in sys.argv, reason='only run for coverage')
def test_cargo_coverage(coverage_ex, request):
    """
    Run cargo tests with coverage enabled
    """
    cov_dir = THIS_DIR / '../.coverage/cargo_test'
    cov_dir.mkdir(parents=True)

    target = os.getenv('TARGET')
    debug_dir = (THIS_DIR / '../target{}/debug/'.format('/' + target if target else '')).resolve()
    path = next(p for p in debug_dir.glob('donkey_make*') if not p.name.endswith('.d'))

    args = coverage_ex, str(cov_dir.resolve()), '--exclude-pattern=/.cargo,/usr/lib', '--verify', str(path)
    p = subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, universal_newlines=True)
    if p.returncode != 0:
        raise RuntimeError('cargo tests failed:\n' + p.stdout)
