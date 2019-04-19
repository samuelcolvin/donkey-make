import json
import os
import re
import subprocess
import sys
from pathlib import Path

import pytest

from .conftest import TPath

THIS_DIR = Path(__file__).parent


def test_help(run):
    p = run('--help')
    assert p.returncode == 0
    assert 'USAGE:' in p.stdout
    assert p.stderr == ''


def test_list_view(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
    - 'echo "this is a test"'
    bar:
      run: x
      description: this is the description
    """)
    p = run()
    assert p.returncode == 0
    assert re.sub(r'v[\d.]+', 'v0.0.0', p.stdout) == (
        'donkey-make v0.0.0, commands available from donkey-make.yaml:\n'
        '  foo - echo "this is a test" (1 line)\n'
        '  bar - this is the description (1 line)\n'
    )
    assert p.stderr == ''


def test_smart_script(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
    - 'echo "this is a test"'
    """)
    p = run('foo')
    assert p.returncode == 0
    assert p.stdout == 'this is a test\n'
    assert re.sub(r'[\d.]+ms', 'XXms', p.stderr) == (
        'Running command "foo" from donkey-make.yaml...\n'
        'foo > echo "this is a test"\n'
        'Command "foo" successful in XXms\n'
    )


def test_tmp_exists(run, test_path: TPath):
    test_path.write_file('.donkey-make.tmp', '.')
    test_path.write_file('donkey-make.yaml', 'foo: xx')
    p = run('foo')
    assert p.returncode == 100
    assert p.stdout == ''
    assert p.stderr == (
        'Error writing temporary file:\n'
        '  .donkey-make.tmp already exists, donkey-make may be running already\n'
    )


def test_no_config_exists(run, test_path):
    p = run()
    assert p.returncode == 100
    assert p.stdout == ''
    assert p.stderr == (
        'No commands config file provided, and no default found, tried:\n'
        '  donk.ya?ml, donkey.ya?ml and donkey-make.ya?ml\n'
    )


def test_invalid_yaml(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', 'foo: 123')
    p = run()
    assert p.returncode == 100
    assert p.stdout == ''
    assert p.stderr == (
        'Error parsing donkey-make.yaml:\n'
        '  invalid type: commands must be a string, sequence, or map at line 1 column 4\n'
    )


def test_subcommands(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    a:
    - echo a
    b:
    - echo b
    c:
    - +a
    - +b
    """)
    p = run('c', combine=True)
    assert p.returncode == 0
    assert re.sub(r'[\d.]+ms', 'XXms', p.stdout) == (
        'Running command "c" from donkey-make.yaml...\n'
        'c > a > echo a\n'
        'a\n'
        'c > b > echo b\n'
        'b\n'
        'Command "c" successful in XXms\n'
    )


def test_fails(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
    - exit 4
    """)
    p = run('foo', combine=True)
    assert p.returncode == 4
    assert re.sub(r'[\d.]+ms', 'XXms', p.stdout) == (
        'Running command "foo" from donkey-make.yaml...\n'
        'foo > exit 4\n'
        'Command "foo" failed in XXms, exit code 4\n'
    )


def test_extra_env(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
      run:
      - import os, json
      - "env = {k: v for k, v in os.environ.items() if k.startswith('DONKEY_')}"
      - print(json.dumps(env))
      ex: python
    bar:
      - +foo
    """)
    p = run('bar')
    assert p.returncode == 0
    env = json.loads(p.stdout)
    assert env == {
        'DONKEY_MAKE_COMMAND': 'bar > foo',
        'DONKEY_MAKE_CONFIG_FILE': '{}/donkey-make.yaml'.format(test_path.path),
        'DONKEY_MAKE_DEPTH': '2',
        'DONKEY_MAKE_KEEP': '0',
    }


@pytest.mark.skipif('--cov' not in sys.argv, reason='only run for coverage')
def test_cargo_coverage(coverage_ex, request):
    """
    Run cargo tests with coverage enabled
    """
    cov_dir = THIS_DIR / '../.coverage/cargo_test'
    cov_dir.mkdir()

    target = os.getenv('TARGET')
    debug_dir = (THIS_DIR / '../target{}/debug/'.format('/' + target if target else '')).resolve()
    path = next(p for p in debug_dir.glob('donkey_make*') if not p.name.endswith('.d'))

    args = coverage_ex, str(cov_dir.resolve()), '--exclude-pattern=/.cargo,/usr/lib', '--verify', str(path)
    p = subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, universal_newlines=True)
    if p.returncode != 0:
        raise RuntimeError('cargo tests failed:\n' + p.stdout)
