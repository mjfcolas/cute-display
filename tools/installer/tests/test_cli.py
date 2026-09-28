import unittest
from contextlib import nullcontext
from unittest import mock

from cute_display_installer import cli
from cute_display_installer.flash.layout import AppImage, Placement, Security, Slot
from test_layout import HABITY_1_1_1, OURS, habity, header, slots

IMAGE = header(OURS.project, OURS.version)


class Install(unittest.TestCase):
    """cli.install against a fake flash: it writes only what the rules allow."""

    def run_install(self, unit, yes=False, answer='n'):
        """cli.install with the real checks, against a device that only reads as `unit`."""
        flash = mock.Mock()
        flash.connect.return_value = nullcontext('esp')
        flash.read_device.return_value = unit
        with mock.patch.object(cli.device, 'connect', flash.connect), \
                mock.patch.object(cli.device, 'read_device', flash.read_device), \
                mock.patch.object(cli.device, 'restart', flash.restart), \
                mock.patch.object(cli.device, 'install', flash.install), \
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
        flash.restart.assert_called_once_with('esp')
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
