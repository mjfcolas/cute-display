import unittest
from contextlib import nullcontext
from unittest import mock

from cute_display_installer import cli
from cute_display_installer.releases import Release
from cute_display_installer.updating import entries
from cute_display_installer.flash.layout import AppImage, Placement, Security, Slot
from test_layout import HABITY_1_1_1, OURS, habity, header, slots

IMAGE = header(OURS.project, OURS.version)
UPDATING = entries(f'# Updating\n\n## {OURS.version}\n\n- The alarm is set again.\n')


class Install(unittest.TestCase):
    """cli.install against a fake flash: it writes only what the rules allow."""

    def run_install(self, unit, yes=False, answer='n', updating=(), oldest_installer=None):
        """cli.install with the real checks, against a device that only reads as `unit`."""
        flash = mock.Mock()
        flash.connect.return_value = nullcontext('esp')
        flash.read_device.return_value = unit
        with mock.patch.object(cli.device, 'connect', flash.connect), \
                mock.patch.object(cli.device, 'read_device', flash.read_device), \
                mock.patch.object(cli.device, 'restart', flash.restart), \
                mock.patch.object(cli.device, 'install', flash.install), \
                mock.patch.object(cli.releases, 'latest_release',
                                  return_value=Release('cute-display.bin', IMAGE, updating, oldest_installer)), \
                mock.patch.object(cli.versions, 'this_installer_version', return_value='2026.9.1'), \
                mock.patch('builtins.input', return_value=answer), \
                mock.patch('builtins.print') as printed:
            try:
                cli.install(None, yes)
            except SystemExit as stop:
                return flash, str(stop)
            finally:
                self.printed = ' '.join(str(call.args) for call in printed.call_args_list)
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

    def test_an_update_with_something_to_know_says_it_then_waits_for_a_yes(self):
        older = habity(booting=Slot.APP1, slots=slots(HABITY_1_1_1, AppImage(OURS.project, '2026.8.0')))
        flash, stop = self.run_install(older, answer='', updating=UPDATING)
        self.assertIn('The alarm is set again.', self.printed)
        flash.install.assert_not_called()
        self.assertIn('not confirmed', stop)

        flash, _ = self.run_install(older, answer='y', updating=UPDATING)
        flash.install.assert_called_once()

        flash, _ = self.run_install(older, yes=True, updating=UPDATING)
        flash.install.assert_called_once()

    def test_an_update_with_nothing_to_know_does_not_ask(self):
        older = habity(booting=Slot.APP1, slots=slots(HABITY_1_1_1, AppImage(OURS.project, '2026.8.0')))
        flash, _ = self.run_install(older, answer='n')
        flash.install.assert_called_once()

    def test_a_release_this_installer_version_is_too_old_for_gets_nothing_written(self):
        flash, stop = self.run_install(habity(), oldest_installer='2026.10.0')
        flash.connect.assert_not_called()
        self.assertIn('needs installer 2026.10.0 or newer', stop)

    def test_a_free_slot_is_used_without_asking(self):
        flash, stop = self.run_install(habity(), answer='n')
        self.assertIsNone(stop)
        flash.install.assert_called_once_with('esp', Placement(Slot.APP1), Slot.APP0, IMAGE)


if __name__ == '__main__':
    unittest.main()
