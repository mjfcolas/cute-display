import unittest

from cute_display_link.console import KeepingLog
from cute_display_link.controls import tap
from cute_display_link_testing.fake_console import FakeConsole
from cute_display_link_testing.fake_controls import FakeControls


class KeptLog(unittest.TestCase):
    def test_the_log_between_the_replies_is_kept_until_taken(self):
        link = KeepingLog(FakeConsole(FakeControls()))
        tap(link, 'yellow')
        tap(link, 'long')
        self.assertEqual(link.take_log(), ['I (1234) app: a log line'] * 2)
        self.assertEqual(link.take_log(), [])
