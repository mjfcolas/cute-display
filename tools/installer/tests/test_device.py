import os
import tempfile
import unittest
from unittest import mock

from esptool.cmds import FatalError

from cute_display_installer.flash import device
from cute_display_installer.flash.layout import OTADATA, Placement, Slot, booting_slot


class Flash:
    """The writes esptool is asked for, in order, and whether the chip was restarted."""

    def __init__(self):
        self.writes = []
        self.restarted = False

    def write_flash(self, esp, addr_data, **_):
        self.writes.extend(addr_data)

    def reset_chip(self, esp, mode):
        self.restarted = True


class Install(unittest.TestCase):
    def install(self, placement, way_back):
        flash = Flash()
        with mock.patch.object(device, 'write_flash', flash.write_flash), \
                mock.patch.object(device, 'reset_chip', flash.reset_chip):
            device.install(None, placement, way_back, b'image')
        return flash

    def test_while_cute_display_is_written_the_bootloader_would_start_habity(self):
        flash = self.install(Placement(Slot.APP1), way_back=Slot.APP0)
        (first, before), (second, image), (third, after) = flash.writes
        self.assertEqual((first, booting_slot(before)), (OTADATA, Slot.APP0))
        self.assertEqual((second, image), (Slot.APP1.offset, b'image'))
        self.assertEqual((third, booting_slot(after)), (OTADATA, Slot.APP1))
        self.assertTrue(flash.restarted)

    def test_with_habity_only_in_factory_the_bootloader_would_start_factory(self):
        flash = self.install(Placement(Slot.APP0), way_back=Slot.FACTORY)
        self.assertEqual(booting_slot(flash.writes[0][1]), Slot.FACTORY)
        self.assertEqual(flash.writes[1][0], Slot.APP0.offset)


class Backup(unittest.TestCase):
    @staticmethod
    def read_flash(esp, address, size, path):
        with open(path, 'wb') as f:
            f.write(b'flash')

    def test_a_backup_appears_once_checked(self):
        with tempfile.TemporaryDirectory() as folder, \
                mock.patch.object(device, 'read_flash', self.read_flash), mock.patch.object(device, 'verify_flash'):
            device.save_flash(None, os.path.join(folder, 'habity-flash.bin'))
            self.assertEqual(os.listdir(folder), ['habity-flash.bin'])

    def test_a_backup_that_does_not_match_the_flash_leaves_nothing(self):
        with tempfile.TemporaryDirectory() as folder, \
                mock.patch.object(device, 'read_flash', self.read_flash), \
                mock.patch.object(device, 'verify_flash', side_effect=FatalError('verify failed')):
            with self.assertRaises(FatalError):
                device.save_flash(None, os.path.join(folder, 'habity-flash.bin'))
            self.assertEqual(os.listdir(folder), [])


if __name__ == '__main__':
    unittest.main()
