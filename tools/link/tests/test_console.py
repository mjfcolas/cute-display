import unittest

from cute_display_link.console import KeepingLog, Request
from cute_display_link.controls import tap
from cute_display_link_testing.fake_console import FakeConsole
from cute_display_link_testing.fake_controls import FakeControls


class StubLink:
    """Returns the lines that a test sets, whatever is written."""

    def __init__(self, *lines):
        self.lines = list(lines)

    def write(self, data):
        self.written = data.decode()

    def flush(self):
        pass

    def readline(self):
        return self.lines.pop(0) if self.lines else b''


class Replies(unittest.TestCase):
    def test_a_reply_in_the_middle_of_a_log_line_is_found_by_its_id(self):
        link = StubLink()
        request = Request(link, 'ls')
        link.lines = [f'I (12) net@@ {request.id} ok\n'.encode(), b'work: up\n']
        self.assertEqual(next(request.replies()), ('ok', ''))


class KeptLog(unittest.TestCase):
    def test_the_log_between_the_replies_is_kept_until_taken(self):
        link = KeepingLog(FakeConsole(FakeControls()))
        tap(link, 'yellow')
        tap(link, 'long')
        self.assertEqual(link.take_log(), ['I (1234) app: a log line'] * 2)
        self.assertEqual(link.take_log(), [])
