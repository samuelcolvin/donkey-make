import os
import shutil
from pathlib import Path
from subprocess import run, CompletedProcess, PIPE, STDOUT

import pytest

THIS_DIR = Path(__file__).parent


def pytest_addoption(parser):
    parser.addoption(
        '--cov', action='store_true', default=False, help='generate coverage'
    )


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


@pytest.fixture(scope='session', name='coverage_dir')
def fix_coverage_dir():
    cov_dir: Path = (THIS_DIR / '../.coverage').resolve()
    if cov_dir.exists():
        shutil.rmtree(cov_dir)
    yield cov_dir

    if cov_dir.exists():
        args = 'kcov', '--merge', 'combined', *map(str, cov_dir.iterdir())
        run(args, check=True, cwd=str(cov_dir))


@pytest.fixture(name='coverage')
def fix_coverage(request, coverage_dir):
    if request.config.getoption('--cov'):
        cov_dir = coverage_dir / request.node.name
        cov_dir.mkdir(parents=True)
        return 'kcov', cov_dir, '--exclude-pattern=/.cargo,/usr/lib'


@pytest.fixture(name='run')
def fix_run(coverage, exe):
    def run_exe(*args, combine=False) -> CompletedProcess:
        env = {k: v for k, v in os.environ.items() if not k.startswith('DONKEY_')}
        kwargs = dict(stdout=PIPE, stderr=PIPE, universal_newlines=True, env=env)
        if combine:
            kwargs['stderr'] = STDOUT

        run_args = str(exe), *args
        if coverage:
            run_args = *coverage, *run_args

        p = run(run_args, **kwargs)
        return p

    return run_exe


class TPath:
    def __init__(self, p: Path):
        self.path = p

    def write_file(self, name, content):
        (self.path / name).write_text(content)

    def __repr__(self):
        return 'TPath(path={})'.format(self.path)


@pytest.fixture(name='test_path')
def fix_test_path(tmp_path: Path):
    prev_cwd = Path.cwd()
    os.chdir(str(tmp_path))
    try:
        yield TPath(tmp_path)
    finally:
        os.chdir(str(prev_cwd))
