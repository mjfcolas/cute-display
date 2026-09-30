import unittest
from contextlib import nullcontext
from unittest import mock

from serial import SerialException

from cute_display_link import remote, usb
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

    def test_what_went_wrong_is_said(self):
        self.assertIn('ms at most', self.run_remote('hold', '20000', 'yellow')[1])
        self.assertIn('No Habity found', self.run_remote('tap', 'long', fails=usb.NoDevice('No Habity found on USB.'))[1])
        self.assertIn('in use', self.run_remote('tap', 'long', fails=SerialException('device busy'))[1])
        self.assertIn('No simulator on nowhere.sock', self.run_remote('--simulator', 'nowhere.sock', 'tap', 'long')[1])
