import pathlib
import subprocess
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).resolve().parents[1] / 'next_version.sh'


class NextVersion(unittest.TestCase):
    """next_version.sh in a repository holding `tags`."""

    def next_version(self, tags, month='2026.9'):
        with tempfile.TemporaryDirectory() as folder:
            def git(*arguments):
                subprocess.run(['git', '-c', 'user.name=t', '-c', 'user.email=t@t', *arguments], cwd=folder,
                               check=True, capture_output=True)
            git('init', '-q')
            git('commit', '-q', '--allow-empty', '-m', 'start')
            for tag in tags:
                git('tag', tag)
            return subprocess.run(['sh', str(SCRIPT), month], cwd=folder, check=True, capture_output=True,
                                  text=True).stdout.strip()

    def test_the_month_s_highest_release_is_followed(self):
        self.assertEqual(self.next_version(['v2026.9.0', 'v2026.9.2', 'v2026.9.1']), '2026.9.3')
        self.assertEqual(self.next_version(['v2026.9.9', 'v2026.9.10']), '2026.9.11')

    def test_a_new_month_starts_at_zero(self):
        self.assertEqual(self.next_version(['v2026.8.4']), '2026.9.0')
        self.assertEqual(self.next_version([]), '2026.9.0')
        self.assertEqual(self.next_version(['v2026.9.0'], month='2026.10'), '2026.10.0')

    def test_tags_that_are_not_releases_are_left_out(self):
        self.assertEqual(self.next_version(['v2026.9.0', 'v2026.9.5-rc1', 'v2026.09.7', 'main-before-rewrite']),
                         '2026.9.1')

    def test_another_month_s_numbers_do_not_mix_in(self):
        self.assertEqual(self.next_version(['v2026.9.3', 'v2026.10.8', 'v2026.1.9'], month='2026.1'), '2026.1.10')


if __name__ == '__main__':
    unittest.main()
