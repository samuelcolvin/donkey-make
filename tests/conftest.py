import os
from pathlib import Path
from subprocess import run, PIPE, STDOUT

import pytest

THIS_DIR = Path(__file__).parent


@pytest.fixture(scope='session')
def exe():
    p = run(['cargo', 'build'], stdout=PIPE, stderr=STDOUT, universal_newlines=True)
    if p.returncode != 0:
        raise RuntimeError('cargo build failed:\n' + p.stdout)
    path = THIS_DIR / '../target/debug/donkey-make'
    assert path.exists()
    return path.resolve()


@pytest.fixture(name='run')
def fix_run(exe):
    def run_exe(*args):
        return run([str(exe), *args], stdout=PIPE, stderr=PIPE, universal_newlines=True)

    return run_exe


class TPath:
    def __init__(self, p: Path):
        self.path = p

    def write_file(self, name, content):
        (self.path / name).write_text(content)


@pytest.fixture(name='test_path')
def fix_test_path(tmp_path: Path):
    prev_cwd = Path.cwd()
    os.chdir(tmp_path)
    try:
        yield TPath(tmp_path)
    finally:
        os.chdir(prev_cwd)