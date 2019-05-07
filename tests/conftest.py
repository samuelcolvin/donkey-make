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


@pytest.fixture(scope='session', name='coverage_ex')
def fix_coverage_ex(request):
    if request.config.getoption('--cov'):
        return str(THIS_DIR / '../.kcov/kcov')


@pytest.fixture(scope='session', name='coverage_dir')
def fix_coverage_dir(coverage_ex):
    if not coverage_ex:
        yield
        return

    cov_dir = THIS_DIR / '../.coverage'
    if cov_dir.exists():
        shutil.rmtree(str(cov_dir.resolve()))
    cov_dir.mkdir()
    yield cov_dir.resolve()

    args = coverage_ex, '--merge', 'combined', *map(str, cov_dir.iterdir())
    run(args, check=True, cwd=str(cov_dir))


@pytest.fixture(name='coverage')
def fix_coverage(request, coverage_dir, coverage_ex):
    if coverage_ex:
        cov_dir = coverage_dir / request.node.name
        cov_dir.mkdir()
        return coverage_ex, str(cov_dir), '--exclude-pattern=/.cargo,/usr/lib'


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
        path = self.path / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)

    def __str__(self):
        return str(self.path)

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
