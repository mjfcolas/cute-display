import tempfile
import unittest
from contextlib import nullcontext
from pathlib import Path
from unittest import mock

from serial import SerialException

from cute_display_link import remote, usb
from cute_display_link.frame import BYTES
from cute_display_link_testing.fake_clock import FakeClock
from cute_display_link_testing.fake_console import FakeConsole
from cute_display_link_testing.fake_controls import FakeControls
from cute_display_link_testing.fake_observation import FakeObservation


class Command(unittest.TestCase):
    def run_remote(self, *arguments, fails=None):
        buttons = FakeControls()
        opened = mock.patch.object(usb, 'open_link', side_effect=fails, return_value=nullcontext(FakeConsole(buttons)))
        with opened, mock.patch('sys.argv', ['remote', *arguments]):
            try:
                remote.main()
            except SystemExit as stop:
                return buttons, str(stop)
        return buttons, None

    def test_each_command_sends_its_control(self):
        for arguments, sent in [(('tap', 'yellow'), 'tap yellow'), (('hold', '1500', 'yellow', 'long'), 'hold 1500 yellow long'),
                                (('turn', '-2'), 'turn -2')]:
            buttons, stop = self.run_remote(*arguments)
            self.assertEqual((buttons.sent, stop), ([sent], None))

    def test_what_is_observed_is_printed(self):
        device = FakeConsole(FakeObservation(lights=(20, 0), playing=True), FakeClock(0))
        printed = []
        with mock.patch.object(usb, 'open_link', return_value=nullcontext(device)), mock.patch('builtins.print', printed.append):
            for arguments in [['lights'], ['sound'], ['clock', '--set', '2026-09-28T06:30:00+02:00']]:
                with mock.patch('sys.argv', ['remote', *arguments]):
                    remote.main()
        self.assertEqual(printed, ['front light 20 %, reading lamp 0 %', 'playing', '2026-09-28T04:30:00+00:00'])

    def test_what_the_screen_says_is_printed_a_line_at_a_time(self):
        device = FakeConsole(FakeObservation(lines=['front radar', 'range 25 km'], times_shown=2))
        printed = []
        with mock.patch.object(usb, 'open_link', return_value=nullcontext(device)), mock.patch('builtins.print', printed.append), \
                mock.patch('sys.argv', ['remote', 'describe']):
            remote.main()
        self.assertEqual(printed, ['front radar\nrange 25 km', 'said when the glass was drawn 2 times'])

    def test_the_screen_is_saved_as_a_png(self):
        with tempfile.TemporaryDirectory() as directory:
            png = Path(directory) / 'screen.png'
            device = FakeConsole(FakeObservation(ink=bytes(BYTES), times_shown=3))
            printed = []
            with mock.patch.object(usb, 'open_link', return_value=nullcontext(device)), mock.patch('builtins.print', printed.append), \
                    mock.patch('sys.argv', ['remote', 'screen', str(png), '--zoom', '1']):
                remote.main()
            self.assertTrue(png.read_bytes().startswith(b'\x89PNG'))
        self.assertEqual(printed, [f'{png}: the glass was drawn 3 times'])

    def test_what_went_wrong_is_said(self):
        self.assertIn('ms at most', self.run_remote('hold', '20000', 'yellow')[1])
        self.assertIn('No Habity found', self.run_remote('tap', 'long', fails=usb.NoDevice('No Habity found on USB.'))[1])
        self.assertIn('in use', self.run_remote('tap', 'long', fails=SerialException('device busy'))[1])
        self.assertIn('No simulator on nowhere.sock', self.run_remote('--simulator', 'nowhere.sock', 'tap', 'long')[1])
