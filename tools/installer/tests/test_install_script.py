import pathlib
import shutil
import stat
import subprocess
import tempfile
import unittest

INSTALLER = pathlib.Path(__file__).resolve().parents[1]
JUSTFILE = INSTALLER.parents[1] / 'justfile'
WHEEL = ('https://github.com/mjfcolas/cute-display/releases/download/v2026.9.1/'
         'cute_display_installer-2026.9.1-py3-none-any.whl')
# A uv that records the wheel it installs, its tools in $HOME/tools.
FAKE_UV = '''#!/bin/sh
case "$1 $2" in
    "tool install") echo "$5" > "$HOME/installed" ;;
    "tool dir") echo "$HOME/tools" ;;
esac
'''
SHELLS = [shell for shell in ('sh', 'dash') if shutil.which(shell)]


def executable(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    path.chmod(path.stat().st_mode | stat.S_IEXEC)


def filled_in(script, wheel):
    """The script as the justfile's `release` recipe fills it in."""
    return script.replace('@WHEEL@', wheel)


class InstallScript(unittest.TestCase):
    """install.sh, filled in, run with a fake uv and no terminal of its own."""

    def setUp(self):
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        self.home = pathlib.Path(folder.name)
        self.bin = self.home / 'bin'
        self.bin.mkdir()
        for tool in ['mkdir', 'cp'] + SHELLS:
            (self.bin / tool).symlink_to(shutil.which(tool))
        executable(self.home / 'tools' / 'cute-display',
                   '#!/bin/sh\nif [ -t 0 ]; then input=terminal; else input=none; fi\necho cute-display "$@" "($input)"\n')
        self.script = self.home / 'install.sh'
        self.script.write_text(filled_in((INSTALLER / 'install.sh').read_text(), WHEEL))

    def run_script(self, *arguments):
        """What the installer printed, run by each shell there is."""
        printed = set()
        for shell in SHELLS:
            done = subprocess.run([str(self.bin / shell), str(self.script), *arguments], stdin=subprocess.DEVNULL,
                                  capture_output=True, text=True, start_new_session=True,
                                  env={'HOME': str(self.home), 'PATH': str(self.bin)})
            self.assertEqual(done.returncode, 0, f'{shell}: {done.stderr}')
            printed.add(done.stdout.strip().splitlines()[-1])
        self.assertEqual(len(printed), 1, printed)
        return printed.pop()

    def installed(self):
        return (self.home / 'installed').read_text().strip()

    def test_the_release_installer_is_installed_then_the_setup_runs(self):
        executable(self.bin / 'uv', FAKE_UV)
        self.assertEqual(self.run_script(), 'cute-display setup (none)')
        self.assertEqual(self.installed(), WHEEL)

    def test_another_command_is_passed_on(self):
        executable(self.bin / 'uv', FAKE_UV)
        self.assertEqual(self.run_script('boot', 'habity'), 'cute-display boot habity (none)')

    def test_without_uv_it_is_installed_then_used(self):
        executable(self.home / 'uv-to-install', FAKE_UV)
        executable(self.bin / 'curl', '#!/bin/sh\n'
                   'echo \'mkdir -p "$HOME/.local/bin" && cp "$HOME/uv-to-install" "$HOME/.local/bin/uv"\'\n')
        self.assertEqual(self.run_script(), 'cute-display setup (none)')
        self.assertEqual(self.installed(), WHEEL)

    def test_uv_that_would_not_install_stops_the_script(self):
        executable(self.bin / 'curl', '#!/bin/sh\necho true\n')
        done = subprocess.run([str(self.bin / 'sh'), str(self.script)], stdin=subprocess.DEVNULL, capture_output=True,
                              text=True, start_new_session=True, env={'HOME': str(self.home), 'PATH': str(self.bin)})
        self.assertNotEqual(done.returncode, 0)
        self.assertIn('uv could not be installed', done.stderr)


class BothScripts(unittest.TestCase):
    def test_both_take_the_wheel_they_are_given_and_no_other(self):
        for name in ('install.sh', 'install.cmd'):
            script = (INSTALLER / name).read_text()
            self.assertEqual(script.count('@WHEEL@'), 1, name)
            self.assertNotIn('.whl', script.replace('@WHEEL@', ''), name)

    def test_the_release_recipe_fills_in_both(self):
        lines = JUSTFILE.read_text().splitlines()
        for name in ('install.sh', 'install.cmd'):
            self.assertTrue(any('s|@WHEEL@|$wheel|g' in line and f'tools/installer/{name} >' in line
                                for line in lines), name)


if __name__ == '__main__':
    unittest.main()
