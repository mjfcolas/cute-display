import unittest
from datetime import datetime, timezone

from cute_display_link import clock
from cute_display_link_testing.fake_clock import FakeClock
from cute_display_link_testing.fake_console import FakeConsole


class Clock(unittest.TestCase):
    def test_the_rtc_is_read_in_utc(self):
        morning = datetime(2026, 9, 28, 4, 30, tzinfo=timezone.utc)
        self.assertEqual(clock.read(FakeConsole(FakeClock(int(morning.timestamp())))), morning)

    def test_the_rtc_set_is_the_rtc_read(self):
        rtc = FakeClock(0)
        device = FakeConsole(rtc)
        morning = datetime(2026, 9, 28, 6, 30).astimezone()
        clock.set_to(device, morning)
        self.assertEqual(rtc.unix_seconds, int(morning.timestamp()))
        self.assertEqual(clock.read(device), morning)
