"""Whole-flash images, each one a case the installer meets, made from a backup of a clock
that updated itself once (factory 1.1.0, app0 1.1.1, app1 empty), and the Cute Display
image for the cases that hold it: `just test-bins <backup>` makes both and calls

  uv run --project tools/installer python tools/test_flashes/make_test_flashes.py <backup> [<Cute Display image>]

They go into test-bins/ beside this file, which git ignores: they hold the backup's Wi-Fi
password.
"""
import hashlib
import os
import struct
import sys

from cute_display_installer.flash.layout import (APP_DESC_OFFSET, APP_DESC_VERSION, APP_SIZE, OTADATA,
                                                 OTADATA_SIZE, PARTITION_ENTRY_SIZE, PARTITION_LABEL,
                                                 PARTITION_MAGIC, PARTITION_TABLE, PARTITION_TABLE_SIZE, Slot,
                                                 otadata_booting)

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'test-bins')
_SEGMENT_COUNT = 1
_IMAGE_HEADER_SIZE, _SEGMENT_HEADER_SIZE = 24, 8
_HASH_APPENDED = 23
_CHECKSUM_SEED = 0xef
# The checksum is the last byte of the 16-byte block the segments end in.
_CHECKSUM_ALIGN = 16
_SHA256_SIZE = 32
_VERSION_IN_IMAGE = slice(APP_DESC_OFFSET + APP_DESC_VERSION.start, APP_DESC_OFFSET + APP_DESC_VERSION.stop)
_PARTITION_MD5_MAGIC = b'\xeb\xeb'
_PARTITION_MD5 = slice(16, 32)
_PARTITION_ADDRESS = 4


class NotInTable(Exception):
    pass


def resealed(image):
    """The app image with its checksum and SHA-256 computed again, trimmed to its end;
    both cover the segments, so any change to them needs this before a bootloader
    accepts the image."""
    image = bytearray(image)
    at, checksum = _IMAGE_HEADER_SIZE, _CHECKSUM_SEED
    for _ in range(image[_SEGMENT_COUNT]):
        _, length = struct.unpack_from('<II', image, at)
        for byte in image[at + _SEGMENT_HEADER_SIZE:at + _SEGMENT_HEADER_SIZE + length]:
            checksum ^= byte
        at += _SEGMENT_HEADER_SIZE + length
    checksum_at = at + _CHECKSUM_ALIGN - 1 - at % _CHECKSUM_ALIGN
    image[checksum_at] = checksum
    end = checksum_at + 1
    if image[_HASH_APPENDED]:
        image[end:end + _SHA256_SIZE] = hashlib.sha256(image[:end]).digest()
        end += _SHA256_SIZE
    return bytes(image[:end])


def with_version(image, version):
    """The app image saying it is `version`."""
    image = bytearray(image)
    image[_VERSION_IN_IMAGE] = version.encode().ljust(APP_DESC_VERSION.stop - APP_DESC_VERSION.start, b'\0')
    return resealed(image)


def with_partition(table, label, offset, size):
    """The partition table with `label` moved, its MD5 entry computed again."""
    table = bytearray(table)
    moved, at = False, 0
    for at in range(0, len(table) - PARTITION_ENTRY_SIZE + 1, PARTITION_ENTRY_SIZE):
        entry = table[at:at + PARTITION_ENTRY_SIZE]
        if entry[:2] != PARTITION_MAGIC:
            break
        if entry[PARTITION_LABEL].split(b'\0')[0] == label.encode():
            struct.pack_into('<II', table, at + _PARTITION_ADDRESS, offset, size)
            moved = True
    if not moved:
        raise NotInTable(f'no partition named {label}')
    if table[at:at + 2] != _PARTITION_MD5_MAGIC:
        raise NotInTable('no MD5 entry after the partitions')
    md5 = slice(at + _PARTITION_MD5.start, at + _PARTITION_MD5.stop)
    table[md5] = hashlib.md5(table[:at]).digest()
    return bytes(table)


def habity_in(backup, slot):
    return resealed(backup[slot.offset:slot.offset + APP_SIZE])


def flash_with(backup, slots=None, booting=None, table=None):
    """The backup with some slots replaced (None erases one), otadata pointing at
    `booting`, and another partition table."""
    flash = bytearray(backup)
    for slot, image in (slots or {}).items():
        flash[slot.offset:slot.offset + APP_SIZE] = (image or b'').ljust(APP_SIZE, b'\xff')
    if booting is not None:
        flash[OTADATA:OTADATA + OTADATA_SIZE] = otadata_booting(booting)
    if table is not None:
        flash[PARTITION_TABLE:PARTITION_TABLE + PARTITION_TABLE_SIZE] = table
    return bytes(flash)


def cases(backup, ours=None):
    """Each case's name and whole flash."""
    habity_1_1_1 = habity_in(backup, Slot.APP0)
    habity_1_1_2 = with_version(habity_1_1_1, '1.1.2')
    table = backup[PARTITION_TABLE:PARTITION_TABLE + PARTITION_TABLE_SIZE]
    made = {
        'updated-once': flash_with(backup, booting=Slot.APP0),
        'never-updated': flash_with(backup, {Slot.APP0: None, Slot.APP1: None}, booting=Slot.FACTORY),
        'updated-twice': flash_with(backup, {Slot.APP1: habity_1_1_2}, booting=Slot.APP1),
        'updated-twice-unreadable': flash_with(backup, {Slot.APP1: with_version(habity_1_1_1, '1.1.2-beta')},
                                               booting=Slot.APP1),
        'other-partition-table': flash_with(backup, booting=Slot.APP0,
                                            table=with_partition(table, 'app1', 0x900000, 0x300000)),
    }
    if ours is not None:
        made['cute-display-in-app0'] = flash_with(backup, {Slot.APP0: ours, Slot.APP1: habity_1_1_2},
                                                  booting=Slot.APP0)
        made['no-habity'] = flash_with(backup, {Slot.FACTORY: None, Slot.APP0: None, Slot.APP1: ours},
                                       booting=Slot.APP1)
    return made


def main(backup_path, ours_path=None):
    with open(backup_path, 'rb') as f:
        backup = f.read()
    ours = None
    if ours_path:
        with open(ours_path, 'rb') as f:
            ours = f.read()
    os.makedirs(OUT, exist_ok=True)
    for name, flash in cases(backup, ours).items():
        with open(os.path.join(OUT, f'{name}.bin'), 'wb') as f:
            f.write(flash)
        print(os.path.join(OUT, f'{name}.bin'))


if __name__ == '__main__':
    if len(sys.argv) not in (2, 3):
        sys.exit(__doc__)
    main(*sys.argv[1:])
