import unittest
from contextlib import nullcontext
from unittest import mock

from serial import SerialException

from cute_display_link import remote, usb
from cute_display_link_testing.fake_card import Card


class Command(unittest.TestCase):
    def run_remote(self, *arguments, fails=None):
        card = Card({})
        opened = mock.patch.object(usb, 'open_link', side_effect=fails, return_value=nullcontext(card))
        with opened, mock.patch('sys.argv', ['remote', *arguments]):
            try:
                remote.main()
            except SystemExit as stop:
                return card, str(stop)
        return card, None

    def test_each_command_sends_its_control(self):
        for arguments, sent in [(('tap', 'yellow'), 'tap yellow'), (('hold', '1500', 'yellow', 'long'), 'hold 1500 yellow long'),
                                (('turn', '-2'), 'turn -2')]:
            card, stop = self.run_remote(*arguments)
            self.assertEqual((card.controls, stop), ([sent], None))

    def test_what_went_wrong_is_said(self):
        self.assertIn('ms at most', self.run_remote('hold', '20000', 'yellow')[1])
        self.assertIn('No Habity found', self.run_remote('tap', 'long', fails=usb.NoDevice('No Habity found on USB.'))[1])
        self.assertIn('in use', self.run_remote('tap', 'long', fails=SerialException('device busy'))[1])
