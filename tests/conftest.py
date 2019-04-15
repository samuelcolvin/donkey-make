import os
from pathlib import Path
from subprocess import run, CompletedProcess, PIPE, STDOUT

import pytest

THIS_DIR = Path(__file__).parent


@pytest.fixture(scope='session')
def exe():
    args = 'cargo', 'build'
    target = os.getenv('TARGET')
    if target:
        args += '--target', target
    p = run(args, stdout=PIPE, stderr=STDOUT, universal_newlines=True)
    if p.returncode != 0:
        raise RuntimeError('cargo build failed:\n' + p.stdout)
    bin_path = THIS_DIR / '../target{}/debug/donkey-make'.format('/' + target if target else '')
    assert bin_path.exists()
    return bin_path.resolve()


@pytest.fixture(name='run')
def fix_run(exe):
    def run_exe(*args) -> CompletedProcess:
        p = run([str(exe), *args], stdout=PIPE, stderr=PIPE, env={'PS4': '+ '}, universal_newlines=True)
        return p

    return run_exe


class TPath:
    def __init__(self, p: Path):
        self.path = p

    def write_file(self, name, content):
        (self.path / name).write_text(content)


@pytest.fixture(name='test_path')
def fix_test_path(tmp_path: Path):
    prev_cwd = Path.cwd()
    os.chdir(str(tmp_path))
    try:
        yield TPath(tmp_path)
    finally:
        os.chdir(str(prev_cwd))
