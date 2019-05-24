import json
import re

from .conftest import TPath


def test_help(run):
    p = run('--help')
    assert p.returncode == 0
    assert 'USAGE:' in p.stdout
    assert p.stderr == ''


def test_list_view(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
    - 'echo "this is a test"'
    - echo more
    bar:
      run: x
      description: this is the description
    very_long:
    - echo this is a long line with more than 40 characters in it
    very_long_name_for_command: echo x
    """)
    p = run()
    assert p.returncode == 0
    assert re.sub(r'v[\d.]+', 'v0.0.0', p.stdout) == (
        'donkey-make v0.0.0, commands available from donkey-make.yaml:\n'
        '  foo            (2 lines) echo "this is a test"…\n'
        '  bar            (1 line) this is the description\n'
        '  very_long      (1 line) echo this is a long line with more than…\n'
        '  very_long_name_for_command (1 line) echo x\n'
    )
    assert p.stderr == ''


def test_smart_script(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
    - 'echo "this is a test"'
    - _echo more
    """)
    p = run('foo')
    assert p.returncode == 0
    assert p.stdout == 'this is a test\nmore\n'
    assert re.sub(r'[\d.]+ms', 'XXms', p.stderr) == (
        'Running command "foo" from donkey-make.yaml...\n'
        '» echo "this is a test"\n'
        'Command "foo" successful in XXms 👍\n'
    )


def test_tmp_exists(run, test_path: TPath):
    test_path.write_file('.donk.tmp', '.')
    test_path.write_file('donkey-make.yaml', 'foo: xx')
    p = run('foo')
    assert p.returncode == 100
    assert p.stdout == ''
    assert p.stderr == (
        'Error writing temporary file:\n'
        '  {}/.donk.tmp already exists, donkey-make may be running already\n'.format(test_path)
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
        '» a › echo a\n'
        'a\n'
        '» b › echo b\n'
        'b\n'
        'Command "c" successful in XXms 👍\n'
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
        '» exit 4\n'
        'Command "foo" failed in XXms, exit code 4 👎\n'
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
        '» exit 4\n'
        'Command "foo" failed in XXms, exit code 4 👎\n'
    )


def test_extra_env(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
      run:
      - import os, json
      - "env = {k: v for k, v in os.environ.items() if k.startswith('DONKEY_')}"
      - print(json.dumps(env))
      ex: python3
    bar:
      - +foo
    """)
    p = run('bar')
    assert p.returncode == 0, (p.stdout, p.stderr)
    env = json.loads(p.stdout)
    assert env == {
        'DONKEY_MAKE_COMMAND': '» foo ›',
        'DONKEY_MAKE_CONFIG_FILE': '{}/donkey-make.yaml'.format(test_path.path),
        'DONKEY_MAKE_DEPTH': '2',
        'DONKEY_MAKE_KEEP': '0',
    }


def test_inline_subcommand(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
    - <bar
    bar:
    - echo this is bar
    """)
    p = run('foo')
    assert p.returncode == 0
    assert p.stdout == 'this is bar\n'
    assert re.sub(r'[\d.]+ms', 'XXms', p.stderr) == (
        'Running command "foo" from donkey-make.yaml...\n'
        '» bar › echo this is bar\n'
        'Command "foo" successful in XXms 👍\n'
    )


def test_inline_env(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
    - <bar
    - echo $foobar
    bar:
    - foobar=apple
    """)
    p = run('foo')
    assert p.returncode == 0
    assert p.stdout == 'apple\n'
    assert re.sub(r'[\d.]+ms', 'XXms', p.stderr) == (
        'Running command "foo" from donkey-make.yaml...\n'
        '» bar › foobar=apple\n'
        '» echo $foobar\n'
        'Command "foo" successful in XXms 👍\n'
    )


def test_inline_missing(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
    - <bar
    """)
    p = run('foo')
    assert p.returncode == 100
    assert p.stdout == ''
    assert re.sub(r'[\d.]+ms', 'XXms', p.stderr) == (
        'Sub-command "bar" not found, commands available are:\n'
        '  foo\n'
    )


def test_inline_repeat(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
    - <foo
    """)
    p = run('foo')
    assert p.returncode == 100
    assert p.stdout == ''
    assert re.sub(r'[\d.]+ms', 'XXms', p.stderr) == (
        'Command "foo" reused in an inline sub-command, this would cause infinite recursion\n'
    )


def test_inline_not_smart(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
    - <bar
    bar:
      run:
        - print(123)
      ex: python
    """)
    p = run('foo')
    assert p.returncode == 100
    assert p.stdout == ''
    assert re.sub(r'[\d.]+ms', 'XXms', p.stderr) == (
        """Sub-command "bar" not a bash-smart script, remove "ex:" or use '+' not '<'\n"""
    )


def test_no_suggestion(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
    - echo testing
    """)
    p = run('spam')
    assert p.returncode == 100
    assert p.stdout == ''
    assert re.sub(r'[\d.]+ms', 'XXms', p.stderr) == (
        'Command "spam" not found, commands available are:\n'
        '  foo\n'
    )


def test_suggestion(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
    - echo testing
    """)
    p = run('fooo')
    assert p.returncode == 100
    assert p.stdout == ''
    assert re.sub(r'[\d.]+ms', 'XXms', p.stderr) == (
        'Command "fooo" not found, commands available are:\n'
        '  foo\n'
        '\n'
        '    perhaps you meant "foo"?\n'
    )


def test_working_dir(run, test_path: TPath):
    test_path.write_file('donkey-make.yaml', """
    foo:
      run:
        - pwd
      working_dir: /tmp/
    """)
    p = run('foo')
    assert p.returncode == 0
    assert p.stdout == '/tmp\n'
    assert re.sub(r'[\d.]+ms', 'XXms', p.stderr) == (
        'Running command "foo" from donkey-make.yaml...\n'
        '» pwd\n'
        'Command "foo" successful in XXms 👍\n'
    )


def test_bash_completion_script(run):
    p = run('--completion-script')
    assert p.returncode == 0
    assert p.stderr == ''
    assert p.stdout.startswith('# donk bash-completion script\n')


def test_bash_command_completion_none(run, test_path: TPath):
    p = run('--complete-command')
    assert p.returncode == 0
    assert p.stderr == ''
    assert p.stdout == ''


def test_bash_command_completion_default(run, test_path: TPath):
    test_path.write_file('donk.yml', """
    foo: xxx
    bar: yyy
    """)
    p = run('--complete-command')
    assert p.returncode == 0
    assert p.stdout == 'foo bar\n'
    assert p.stderr == ''


def test_bash_command_completion_custom_file(run, test_path: TPath):
    test_path.write_file('other/dir/donk.yml', """
    a: xxx
    b: yyy
    """)
    p = run('--complete-command', './other/dir/donk.yml')
    assert p.returncode == 0
    assert p.stdout == 'a b\n'
    assert p.stderr == ''


def test_bash_command_completion_custom_file_missing(run, test_path: TPath):
    p = run('--complete-command', './other/dir/donk.yml')
    assert p.returncode == 0
    assert p.stdout == ''
    assert p.stderr == ''
