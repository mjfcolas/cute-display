import struct
import unittest

from flash_layout import (APP_HEADER_SIZE, APP_SIZE, OTADATA_SECTOR, AppImage, Device, Partition, Security, Slot,
                          _OTA_IMG_INVALID, _OTA_STATE_OFFSET, app_image, booting_slot, image_refusals,
                          otadata_booting, partitions)

# The first entry of the otadata this project's unit came with, running Habity 1.1.1 from app0.
STOCK_OTADATA_ENTRY = bytes.fromhex('01000000' + 'ff' * 20 + '02000000' + '9a984347')

HABITY_TABLE = [
    ('nvs', 1, 0x02, 0x9000, 0x10000),
    ('otadata', 1, 0x00, 0x19000, 0x2000),
    ('phy_init', 1, 0x01, 0x1b000, 0x5000),
    ('factory', 0, 0x00, 0x20000, 0x400000),
    ('app0', 0, 0x10, 0x420000, 0x400000),
    ('app1', 0, 0x11, 0x820000, 0x400000),
    ('coredump', 1, 0x03, 0xc20000, 0x3e0000),
]

HABITY_1_1_0, HABITY_1_1_1 = AppImage('habity', '1.1.0'), AppImage('habity', '1.1.1')
OURS = AppImage('cute-display', '2026.9.0')
OPEN = Security(secure_boot=False, flash_encryption=False)


def table(rows):
    entries = b''.join(b'\xaa\x50' + bytes([t, s]) + struct.pack('<II', o, n) + label.encode().ljust(16, b'\0')
                       + b'\0' * 4 for label, t, s, o, n in rows)
    return entries.ljust(0xc00, b'\xff')


def header(project, version):
    desc = struct.pack('<I', 0xabcd5432) + b'\0' * 12 + version.encode().ljust(32, b'\0') + project.encode().ljust(32, b'\0')
    return (b'\xe9' + b'\0' * 31 + desc).ljust(APP_HEADER_SIZE, b'\0')


def habity(booting=Slot.APP0, slots=None, rows=HABITY_TABLE, security=OPEN):
    slots = slots if slots is not None else {Slot.FACTORY: HABITY_1_1_0, Slot.APP0: HABITY_1_1_1, Slot.APP1: None}
    return Device(security, [Partition(*r) for r in rows], booting, slots)


class Partitions(unittest.TestCase):
    def test_the_table_is_read_up_to_its_first_blank_entry(self):
        found = partitions(table(HABITY_TABLE))
        self.assertEqual([p.label for p in found], [r[0] for r in HABITY_TABLE])
        self.assertEqual(found[5], Partition('app1', 0, 0x11, 0x820000, 0x400000))


class AppImages(unittest.TestCase):
    def test_a_slot_says_which_project_and_version_it_holds(self):
        self.assertEqual(app_image(header('habity', '1.1.1')), HABITY_1_1_1)
        self.assertTrue(app_image(header('habity', '1.1.1')).is_stock)
        self.assertFalse(app_image(header('libespidf', 'c62cace')).is_stock)

    def test_an_erased_slot_holds_nothing(self):
        self.assertIsNone(app_image(b'\xff' * APP_HEADER_SIZE))

    def test_cute_display_may_go_into_app1(self):
        self.assertEqual(image_refusals(header('cute-display', '2026.9.0')), [])

    def test_what_is_not_cute_display_does_not_go_into_app1(self):
        self.assertIn('not an app image', ' '.join(image_refusals(b'\xff' * APP_HEADER_SIZE)))
        self.assertIn('it says Habity 1.1.1', ' '.join(image_refusals(header('habity', '1.1.1'))))
        self.assertIn('it says libespidf', ' '.join(image_refusals(header('libespidf', 'c62cace'))))
        undescribed = (b'\xe9' + b'\0' * 31).ljust(APP_HEADER_SIZE, b'\0')
        self.assertIn('does not say what it is', ' '.join(image_refusals(undescribed)))
        too_large = header('cute-display', '2026.9.0').ljust(APP_SIZE + 1, b'\0')
        self.assertIn('larger than app1', ' '.join(image_refusals(too_large)))

    def test_an_image_without_a_description_is_not_taken_for_habity(self):
        image = app_image((b'\xe9' + b'\0' * 31).ljust(APP_HEADER_SIZE, b'\0'))
        self.assertIsNotNone(image)
        self.assertFalse(image.is_stock)


class Otadata(unittest.TestCase):
    def test_blank_otadata_boots_factory(self):
        self.assertEqual(booting_slot(b'\xff' * 0x2000), Slot.FACTORY)
        self.assertEqual(otadata_booting(Slot.FACTORY), b'\xff' * 0x2000)

    def test_the_written_otadata_boots_the_slot_it_was_written_for(self):
        for slot in Slot:
            self.assertEqual(booting_slot(otadata_booting(slot)), slot)

    def test_booting_app0_is_the_otadata_the_unit_came_with(self):
        self.assertEqual(otadata_booting(Slot.APP0)[:32], STOCK_OTADATA_ENTRY)
        self.assertEqual(booting_slot(STOCK_OTADATA_ENTRY.ljust(0x2000, b'\xff')), Slot.APP0)

    def test_the_highest_valid_sequence_wins(self):
        both = otadata_booting(Slot.APP0)[:OTADATA_SECTOR] + otadata_booting(Slot.APP1)[:OTADATA_SECTOR]
        self.assertEqual(booting_slot(both), Slot.APP1)

    def test_an_entry_with_a_bad_crc_is_ignored(self):
        broken = bytearray(otadata_booting(Slot.APP1))
        broken[28] ^= 0xff
        self.assertEqual(booting_slot(bytes(broken)), Slot.FACTORY)

    def test_an_entry_marked_invalid_is_ignored(self):
        invalid = bytearray(otadata_booting(Slot.APP1))
        invalid[_OTA_STATE_OFFSET] = _OTA_IMG_INVALID
        self.assertEqual(booting_slot(bytes(invalid)), Slot.FACTORY)


class Installing(unittest.TestCase):
    def test_the_unit_as_it_came_may_take_cute_display(self):
        device = habity()
        self.assertEqual(device.install_refusals(), [])
        self.assertEqual(device.install_warnings(), [])
        self.assertEqual(device.stock_slot(), Slot.APP0)

    def test_cute_display_may_replace_itself(self):
        self.assertEqual(habity(booting=Slot.APP1, slots={Slot.FACTORY: HABITY_1_1_0, Slot.APP0: HABITY_1_1_1, Slot.APP1: OURS})
                         .install_refusals(), [])

    def test_a_device_never_updated_goes_back_to_factory(self):
        device = habity(booting=Slot.FACTORY, slots={Slot.FACTORY: HABITY_1_1_0, Slot.APP0: None, Slot.APP1: None})
        self.assertEqual(device.install_refusals(), [])
        self.assertEqual(device.stock_slot(), Slot.FACTORY)

    def test_habity_running_from_app1_is_not_erased(self):
        device = habity(booting=Slot.APP1, slots={Slot.FACTORY: HABITY_1_1_0, Slot.APP0: HABITY_1_1_1,
                                               Slot.APP1: AppImage('habity', '1.1.2')})
        self.assertIn('runs from app1', ' '.join(device.install_refusals()))

    def test_secured_devices_are_refused(self):
        for security in (Security(True, False), Security(False, True)):
            self.assertIn('secure boot', ' '.join(habity(security=security).install_refusals()))

    def test_another_partition_table_is_refused(self):
        moved = [r if r[0] != 'app1' else ('app1', 0, 0x11, 0x900000, 0x300000) for r in HABITY_TABLE]
        self.assertIn('expected app1 at 0x820000', ' '.join(habity(rows=moved).install_refusals()))
        without = [r for r in HABITY_TABLE if r[0] != 'otadata']
        self.assertIn('expected otadata', ' '.join(habity(rows=without).install_refusals()))

    def test_without_habity_there_is_no_way_back(self):
        device = habity(booting=Slot.APP1, slots={Slot.FACTORY: None, Slot.APP0: OURS, Slot.APP1: OURS})
        self.assertIsNone(device.stock_slot())
        self.assertIn('no way back', ' '.join(device.install_refusals()))

    def test_booting_another_slot_needs_the_known_layout_only(self):
        running_from_app1 = habity(booting=Slot.APP1, slots={Slot.FACTORY: HABITY_1_1_0, Slot.APP0: HABITY_1_1_1,
                                                          Slot.APP1: AppImage('habity', '1.1.2')})
        self.assertEqual(running_from_app1.boot_refusals(Slot.FACTORY), [])
        self.assertNotEqual(habity(security=Security(True, False)).boot_refusals(Slot.FACTORY), [])

    def test_an_empty_slot_is_not_booted(self):
        self.assertIn('app1 is empty', ' '.join(habity().boot_refusals(Slot.APP1)))

    def test_an_untried_habity_version_is_a_warning(self):
        device = habity(slots={Slot.FACTORY: HABITY_1_1_0, Slot.APP0: AppImage('habity', '2.0.0'), Slot.APP1: None})
        self.assertEqual(device.install_refusals(), [])
        self.assertIn('Habity 2.0.0', ' '.join(device.install_warnings()))


if __name__ == '__main__':
    unittest.main()
