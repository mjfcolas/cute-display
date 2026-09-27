import hashlib
import pathlib
import struct
import sys
import unittest

from esptool.bin_image import LoadFirmwareImage

from cute_display_installer.flash.layout import (APP_HEADER_SIZE, FLASH_SIZE, OTADATA, OTADATA_SIZE,
                                                 PARTITION_TABLE, PARTITION_TABLE_SIZE, Device, Placement,
                                                 Security, Slot, Target, app_image, booting_slot, partitions)
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
from make_test_flashes import NotInTable, cases, resealed, with_partition, with_version  # noqa: E402


def image(project, version):
    """A small app image as ESP-IDF builds them: one segment, its description first,
    checksum and SHA-256 appended."""
    desc = (struct.pack('<I', 0xabcd5432) + b'\0' * 12 + version.encode().ljust(32, b'\0')
            + project.encode().ljust(32, b'\0')).ljust(256, b'\0') + bytes(range(256))
    header = bytes([0xe9, 1, 2, 0x2f]) + struct.pack('<I', 0x40380000) + b'\xee\0\0\0\0\0\0\0\0\0\0\0\0\0\0\1'
    return resealed(header + struct.pack('<II', 0x3c000020, len(desc)) + desc + b'\0' * 48)


HABITY_TABLE = [
    ('nvs', 1, 0x02, 0x9000, 0x10000),
    ('otadata', 1, 0x00, 0x19000, 0x2000),
    ('phy_init', 1, 0x01, 0x1b000, 0x5000),
    ('factory', 0, 0x00, 0x20000, 0x400000),
    ('app0', 0, 0x10, 0x420000, 0x400000),
    ('app1', 0, 0x11, 0x820000, 0x400000),
    ('coredump', 1, 0x03, 0xc20000, 0x3e0000),
]


def habity_table():
    """The table ESP-IDF writes: the entries, then one holding their MD5."""
    rows = b''.join(b'\xaa\x50' + bytes([t, s]) + struct.pack('<II', o, n) + label.encode().ljust(16, b'\0') + b'\0' * 4
                    for label, t, s, o, n in HABITY_TABLE)
    return (rows + b'\xeb\xeb' + b'\xff' * 14 + hashlib.md5(rows).digest()).ljust(PARTITION_TABLE_SIZE, b'\xff')


def backup():
    """A clock that updated itself once: factory 1.1.0, app0 1.1.1, app1 empty."""
    flash = bytearray(b'\xff' * FLASH_SIZE)
    flash[PARTITION_TABLE:PARTITION_TABLE + PARTITION_TABLE_SIZE] = habity_table()
    for slot, version in ((Slot.FACTORY, '1.1.0'), (Slot.APP0, '1.1.1')):
        made = image('habity', version)
        flash[slot.offset:slot.offset + len(made)] = made
    return bytes(flash)


def read(flash):
    return Device(Security(False, False), partitions(flash[PARTITION_TABLE:PARTITION_TABLE + PARTITION_TABLE_SIZE]),
                  booting_slot(flash[OTADATA:OTADATA + OTADATA_SIZE]),
                  {slot: app_image(flash[slot.offset:slot.offset + APP_HEADER_SIZE]) for slot in Slot})


class Images(unittest.TestCase):
    def test_a_resealed_image_passes_esptool_checks(self):
        loaded = LoadFirmwareImage('esp32s3', with_version(image('habity', '1.1.1'), '1.1.2'))
        self.assertEqual(loaded.calculate_checksum(), loaded.checksum)
        self.assertEqual(loaded.stored_digest, loaded.calc_digest)

    def test_resealing_an_intact_image_changes_nothing(self):
        intact = image('habity', '1.1.1')
        self.assertEqual(resealed(intact.ljust(0x1000, b'\xff')), intact)

    def test_a_new_version_is_what_the_image_says(self):
        self.assertEqual(str(app_image(with_version(image('habity', '1.1.1'), '1.1.2'))), 'Habity 1.1.2')

    def test_a_moved_partition_keeps_the_table_whole(self):
        moved = with_partition(habity_table(), 'app1', 0x900000, 0x300000)
        app1 = next(p for p in partitions(moved) if p.label == 'app1')
        self.assertEqual((app1.offset, app1.size), (0x900000, 0x300000))
        rows = moved[:len(HABITY_TABLE) * 32]
        self.assertEqual(moved[len(rows) + 16:len(rows) + 32], hashlib.md5(rows).digest())

    def test_a_partition_not_there_is_not_silently_skipped(self):
        with self.assertRaisesRegex(NotInTable, 'no partition named app2'):
            with_partition(habity_table(), 'app2', 0x900000, 0x300000)

    def test_a_table_without_its_md5_is_refused(self):
        without_md5 = habity_table()[:len(HABITY_TABLE) * 32].ljust(PARTITION_TABLE_SIZE, b'\xff')
        with self.assertRaisesRegex(NotInTable, 'no MD5 entry'):
            with_partition(without_md5, 'app1', 0x900000, 0x300000)


class Cases(unittest.TestCase):
    """Each case is what its name says, as the installer reads it."""

    @classmethod
    def setUpClass(cls):
        cls.made = {name: read(flash) for name, flash in
                    cases(backup(), image('cute-display', '2026.9.0')).items()}

    def test_updated_once_takes_cute_display_in_app1(self):
        self.assertEqual(self.made['updated-once'].placement(), Placement(Slot.APP1))

    def test_never_updated_takes_it_in_app0_and_goes_back_to_factory(self):
        unit = self.made['never-updated']
        self.assertEqual((unit.booting, unit.placement()), (Slot.FACTORY, Placement(Slot.APP0)))
        self.assertEqual(unit.slot_for(Target.HABITY), Slot.FACTORY)

    def test_updated_twice_erases_the_older_habity(self):
        unit = self.made['updated-twice']
        self.assertEqual(unit.placement(), Placement(Slot.APP0, erases=unit.slots[Slot.APP0]))
        self.assertEqual(str(unit.slots[Slot.APP1]), 'Habity 1.1.2')

    def test_an_unreadable_version_is_refused(self):
        self.assertIn('cannot be told', ' '.join(self.made['updated-twice-unreadable'].install_refusals()))

    def test_another_partition_table_is_refused(self):
        self.assertIn('expected app1 at 0x820000', ' '.join(self.made['other-partition-table'].install_refusals()))

    def test_cute_display_in_app0_is_updated_there(self):
        unit = self.made['cute-display-in-app0']
        self.assertEqual((unit.placement(), unit.slot_for(Target.HABITY)), (Placement(Slot.APP0), Slot.APP1))

    def test_without_habity_there_is_no_way_back(self):
        self.assertIn('no way back', ' '.join(self.made['no-habity'].install_refusals()))


if __name__ == '__main__':
    unittest.main()
