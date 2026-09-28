import struct
import unittest

from cute_display_installer.flash.layout import (
    _OTA_IMG_INVALID, _OTA_STATE_OFFSET, APP_HEADER_SIZE, APP_SIZE, OTADATA_SECTOR, AppImage, Device, Partition,
    Placement, Security, Slot, Target, app_image, booting_slot, image_refusals, otadata_booting, partitions)

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

    def test_each_image_is_named_as_people_know_it(self):
        self.assertEqual(str(HABITY_1_1_1), 'Habity 1.1.1')
        self.assertEqual(str(OURS), 'Cute Display 2026.9.0')
        self.assertEqual(str(app_image(header('libespidf', 'c62cace'))), 'libespidf c62cace')

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
        self.assertIn('larger than a slot', ' '.join(image_refusals(too_large)))

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


HABITY_1_1_2 = AppImage('habity', '1.1.2')


def slots(app0, app1, factory=HABITY_1_1_0):
    return {Slot.FACTORY: factory, Slot.APP0: app0, Slot.APP1: app1}


class Placing(unittest.TestCase):
    def test_the_clock_as_it_came_takes_cute_display_in_its_empty_slot(self):
        device = habity()
        self.assertEqual(device.install_refusals(), [])
        self.assertEqual(device.placement(), Placement(Slot.APP1))
        self.assertEqual(device.habity(besides=Slot.APP1), Slot.APP0)

    def test_cute_display_stays_where_it_is(self):
        for booting in (Slot.APP0, Slot.APP1):
            device = habity(booting=booting, slots=slots(OURS, HABITY_1_1_1))
            self.assertEqual(device.placement(), Placement(Slot.APP0))

    def test_a_slot_without_habity_is_free(self):
        device = habity(slots=slots(AppImage('libespidf', 'c62cace'), HABITY_1_1_1))
        self.assertEqual(device.placement(), Placement(Slot.APP0))

    def test_with_habity_in_both_the_older_one_is_erased(self):
        for app0, app1, older in ((HABITY_1_1_1, HABITY_1_1_2, Slot.APP0), (HABITY_1_1_2, HABITY_1_1_1, Slot.APP1)):
            device = habity(booting=Slot.APP1, slots=slots(app0, app1))
            self.assertEqual(device.install_refusals(), [])
            self.assertEqual(device.placement(), Placement(older, erases=device.slots[older]))

    def test_only_a_release_reads_as_numbers(self):
        self.assertEqual(AppImage('cute-display', '2026.9.0').release, (2026, 9, 0))
        for version in ('2026.9.1-snapshot', '2026.9.0-4-ged99824', '2026.9.0-dirty', '0.0.0-untagged', '1.2-beta', ''):
            self.assertIsNone(AppImage('cute-display', version).release, version)

    def test_versions_are_compared_as_numbers(self):
        device = habity(slots=slots(AppImage('habity', '1.10.0'), AppImage('habity', '1.9.0')))
        self.assertEqual(device.placement().slot, Slot.APP1)

    def test_a_version_that_does_not_read_is_not_guessed(self):
        device = habity(slots=slots(HABITY_1_1_1, AppImage('habity', '1.2-beta')))
        self.assertIsNone(device.placement())
        self.assertIn('cannot be told', ' '.join(device.install_refusals()))

    def test_a_clock_never_updated_goes_back_to_factory(self):
        device = habity(booting=Slot.FACTORY, slots=slots(None, None))
        self.assertEqual(device.install_refusals(), [])
        self.assertEqual(device.habity(besides=device.placement().slot), Slot.FACTORY)

    def test_without_habity_there_is_no_way_back(self):
        device = habity(booting=Slot.APP1, slots=slots(OURS, None, factory=None))
        self.assertIn('no way back', ' '.join(device.install_refusals()))

    def test_secured_devices_are_refused(self):
        for security in (Security(True, False), Security(False, True)):
            self.assertIn('secure boot', ' '.join(habity(security=security).install_refusals()))

    def test_another_partition_table_is_refused(self):
        moved = [r if r[0] != 'app1' else ('app1', 0, 0x11, 0x900000, 0x300000) for r in HABITY_TABLE]
        self.assertIn('expected app1 at 0x820000', ' '.join(habity(rows=moved).install_refusals()))
        without = [r for r in HABITY_TABLE if r[0] != 'otadata']
        self.assertIn('expected otadata', ' '.join(habity(rows=without).install_refusals()))


class Booting(unittest.TestCase):
    def test_names_lead_to_their_slots(self):
        device = habity(booting=Slot.APP0, slots=slots(OURS, HABITY_1_1_2))
        self.assertEqual({target: device.slot_for(target) for target in Target}, {
            Target.HABITY: Slot.APP1, Target.FACTORY: Slot.FACTORY, Target.CUTE_DISPLAY: Slot.APP0,
            Target.APP0: Slot.APP0, Target.APP1: Slot.APP1})

    def test_habity_is_its_newest_firmware_else_the_factory_one(self):
        self.assertEqual(habity(slots=slots(HABITY_1_1_2, HABITY_1_1_1)).slot_for(Target.HABITY), Slot.APP0)
        self.assertEqual(habity(slots=slots(OURS, None)).slot_for(Target.HABITY), Slot.FACTORY)

    def test_what_is_not_there_is_not_started(self):
        without_us = habity()
        self.assertIn('no cute-display to start', ' '.join(without_us.boot_refusals(Target.CUTE_DISPLAY)))
        self.assertIn('app1 is empty', ' '.join(without_us.boot_refusals(Target.APP1)))

    def test_booting_needs_the_known_layout_only(self):
        self.assertEqual(habity(booting=Slot.APP1, slots=slots(HABITY_1_1_1, HABITY_1_1_2))
                         .boot_refusals(Target.FACTORY), [])
        self.assertNotEqual(habity(security=Security(True, False)).boot_refusals(Target.FACTORY), [])


if __name__ == '__main__':
    unittest.main()
