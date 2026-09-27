import unittest
from contextlib import nullcontext
from unittest import mock

from cute_display_installer import cli
from cute_display_installer.flash.layout import AppImage, Placement, Security, Slot
from test_layout import HABITY_1_1_1, OURS, habity, header, slots

IMAGE = header(OURS.project, OURS.version)


class Explain(unittest.TestCase):
    def test_a_busy_port_asks_for_the_monitor_to_be_closed(self):
        for message in ("could not open port /dev/ttyACM0: [Errno 16] Device or resource busy",
                        "could not open port 'COM7': PermissionError(13, 'Access is denied.', None, 5)"):
            self.assertIn('close any serial monitor', cli.explain(Exception(message)))

    def test_a_forbidden_port_asks_for_the_group(self):
        self.assertIn('dialout or uucp', cli.explain(Exception('[Errno 13] Permission denied: /dev/ttyACM0')))

    def test_anything_else_asks_for_the_cable_again(self):
        self.assertIn('Unplug it', cli.explain(Exception('Failed to connect to ESP32-S3: No serial data received.')))


class Install(unittest.TestCase):
    """cli.install against a fake flash: it writes only what the rules allow."""

    def run_install(self, unit, yes=False, answer='n'):
        flash = mock.Mock()
        flash.connect.return_value = nullcontext('esp')
        flash.read_device.return_value = unit
        with mock.patch.object(cli, 'device', flash), \
                mock.patch.object(cli.releases, 'latest_image', return_value=('cute-display.bin', IMAGE)), \
                mock.patch('builtins.input', return_value=answer), \
                mock.patch('builtins.print'):
            try:
                cli.install(None, yes)
            except SystemExit as stop:
                return flash, str(stop)
        return flash, None

    def test_a_refused_device_gets_nothing_written(self):
        flash, stop = self.run_install(habity(security=Security(True, False)))
        flash.install.assert_not_called()
        self.assertIn('secure boot', stop)

    def test_erasing_a_habity_firmware_waits_for_a_yes(self):
        both = habity(booting=Slot.APP1, slots=slots(HABITY_1_1_1, AppImage('habity', '1.1.2')))
        flash, stop = self.run_install(both, answer='')
        flash.install.assert_not_called()
        self.assertIn('not confirmed', stop)

        flash, _ = self.run_install(both, answer='y')
        flash.install.assert_called_once_with('esp', Placement(Slot.APP0, HABITY_1_1_1), Slot.APP1, IMAGE)

        flash, _ = self.run_install(both, yes=True, answer='n')
        flash.install.assert_called_once()

    def test_a_free_slot_is_used_without_asking(self):
        flash, stop = self.run_install(habity(), answer='n')
        self.assertIsNone(stop)
        flash.install.assert_called_once_with('esp', Placement(Slot.APP1), Slot.APP0, IMAGE)


if __name__ == '__main__':
    unittest.main()
