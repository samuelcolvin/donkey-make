from .conftest import TPath


def test_help(run):
    p = run('--help')
    assert p.returncode == 0
    assert 'USAGE:' in p.stdout
    assert p.stderr == ''


def test_simple(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
    - 'echo "this is a test"'
    """)
    p = run()
    assert p.returncode == 0
    assert p.stdout == (
        'Running command "foo" from "donkey-make.yaml"...\n'
        'this is a test\n'
        'Command "foo" successful\n'
    )
    assert p.stderr == ''


def test_tmp_exists(run, test_path: TPath):
    test_path.write_file('~donkey-make.tmp', '.')
    p = run()
    assert p.returncode == 1
    assert p.stdout == ''
    assert p.stderr == (
        'Error opening donkey-make.yaml:\n'
        '  No such file or directory (os error 2)\n'
    )


def test_invalid_yaml(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', 'foo: string')
    p = run()
    assert p.returncode == 1
    assert p.stdout == ''
    assert p.stderr == (
        'Error parsing donkey-make.yaml:\n'
        '  invalid type: string "string", expected struct Command at line 1 column 4\n'
    )
