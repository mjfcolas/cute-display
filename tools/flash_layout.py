"""The Habity's flash: its partitions, which app the bootloader starts, what each app slot
holds, and whether cute-display can go in app1 and come back out.

Everything here works on bytes read from the flash; tools/cute_display.py reads and
writes them. Only app1 and otadata are ever written: Habity's firmware stays in factory
and app0, and going back to it is a matter of otadata alone.
"""
import struct
import zlib
from dataclasses import dataclass
from enum import Enum

FLASH_SIZE = 0x1000000
PARTITION_TABLE = 0x8000
PARTITION_TABLE_SIZE = 0xc00
OTADATA = 0x19000
OTADATA_SIZE = 0x2000
OTADATA_SECTOR = 0x1000
APP_SIZE = 0x400000

STOCK_PROJECT = 'habity'
TESTED_STOCK_VERSIONS = {'1.1.0', '1.1.1'}

_APP_TYPE, _DATA_TYPE = 0, 1
_OTADATA_SUBTYPE = 0x00


class Slot(Enum):
    """An app partition: its label, subtype and offset."""
    FACTORY = ('factory', 0x00, 0x20000)
    APP0 = ('app0', 0x10, 0x420000)
    APP1 = ('app1', 0x11, 0x820000)

    def __init__(self, label, subtype, offset):
        self.label = label
        self.subtype = subtype
        self.offset = offset

    def __str__(self):
        return self.label

    @classmethod
    def named(cls, label):
        for slot in cls:
            if slot.label == label:
                return slot
        raise ValueError(f'no slot named {label}')


OTA_SLOTS_BY_SEQUENCE = [Slot.APP0, Slot.APP1]


@dataclass(frozen=True)
class Partition:
    label: str
    type: int
    subtype: int
    offset: int
    size: int


_EXPECTED_PARTITIONS = [Partition('otadata', _DATA_TYPE, _OTADATA_SUBTYPE, OTADATA, OTADATA_SIZE)] + [
    Partition(slot.label, _APP_TYPE, slot.subtype, slot.offset, APP_SIZE) for slot in Slot
]

_PARTITION_MAGIC = b'\xaa\x50'
_PARTITION_ENTRY_SIZE = 32
_PARTITION_LABEL = slice(12, 28)

_IMAGE_MAGIC = 0xe9
# esp_app_desc_t sits right after the image header and its first segment's header.
_APP_DESC_OFFSET = 0x20
_APP_DESC_SIZE = 0x100
_APP_DESC_MAGIC = 0xabcd5432
_APP_DESC_VERSION = slice(16, 48)
_APP_DESC_PROJECT = slice(48, 80)
APP_HEADER_SIZE = _APP_DESC_OFFSET + _APP_DESC_SIZE

_OTA_SEQ_BLANK = 0xffffffff
_OTA_STATE_OFFSET = 24
_OTA_IMG_VALID, _OTA_IMG_INVALID, _OTA_IMG_ABORTED = 2, 3, 4
_OTA_STATES_REFUSED = {_OTA_IMG_INVALID, _OTA_IMG_ABORTED}


@dataclass(frozen=True)
class AppImage:
    project: str
    version: str

    @property
    def is_stock(self):
        return self.project == STOCK_PROJECT

    def __str__(self):
        return f'Habity {self.version}' if self.is_stock else f'{self.project} {self.version}'


UNDESCRIBED = AppImage(project='unknown', version='unknown')


@dataclass(frozen=True)
class Security:
    secure_boot: bool
    flash_encryption: bool


def partitions(table):
    found = []
    for at in range(0, len(table) - _PARTITION_ENTRY_SIZE + 1, _PARTITION_ENTRY_SIZE):
        entry = table[at:at + _PARTITION_ENTRY_SIZE]
        if entry[:2] != _PARTITION_MAGIC:
            break
        offset, size = struct.unpack_from('<II', entry, 4)
        found.append(Partition(_c_string(entry[_PARTITION_LABEL]), entry[2], entry[3], offset, size))
    return found


def app_image(header):
    """What an app slot holds, from its first APP_HEADER_SIZE bytes; None when it holds
    no image."""
    if len(header) < APP_HEADER_SIZE or header[0] != _IMAGE_MAGIC:
        return None
    desc = header[_APP_DESC_OFFSET:]
    if struct.unpack_from('<I', desc)[0] != _APP_DESC_MAGIC:
        return UNDESCRIBED
    return AppImage(project=_c_string(desc[_APP_DESC_PROJECT]), version=_c_string(desc[_APP_DESC_VERSION]))


def image_refusals(contents):
    """Why a file may not go into app1; empty when it may."""
    held = app_image(contents[:APP_HEADER_SIZE])
    if held is None:
        return ['it is not an app image.']
    refusals = []
    if held == UNDESCRIBED:
        refusals.append('the image does not say what it is.')
    if held.is_stock:
        refusals.append(f"it is Habity's firmware ({held}).")
    if len(contents) > APP_SIZE:
        refusals.append(f'it is larger than app1 ({len(contents) // 1024} KB, app1 holds {APP_SIZE // 1024}).')
    return refusals


def _c_string(field):
    return field.split(b'\0')[0].decode(errors='replace')


def booting_slot(otadata):
    """The slot the bootloader starts: the OTA slot of the highest valid sequence
    number, factory without one."""
    best = None
    for at in (0, OTADATA_SECTOR):
        seq, = struct.unpack_from('<I', otadata, at)
        state, crc = struct.unpack_from('<II', otadata, at + _OTA_STATE_OFFSET)
        if seq == _OTA_SEQ_BLANK or crc != _sequence_crc(seq) or state in _OTA_STATES_REFUSED:
            continue
        best = seq if best is None else max(best, seq)
    return Slot.FACTORY if best is None else OTA_SLOTS_BY_SEQUENCE[(best - 1) % len(OTA_SLOTS_BY_SEQUENCE)]


def otadata_booting(slot):
    """The whole otadata partition, written so that the bootloader starts this slot."""
    if slot == Slot.FACTORY:
        return b'\xff' * OTADATA_SIZE
    seq = OTA_SLOTS_BY_SEQUENCE.index(slot) + 1
    entry = struct.pack('<I', seq).ljust(_OTA_STATE_OFFSET, b'\xff') + struct.pack('<II', _OTA_IMG_VALID, _sequence_crc(seq))
    return entry.ljust(OTADATA_SIZE, b'\xff')


def _sequence_crc(seq):
    return zlib.crc32(struct.pack('<I', seq), 0xffffffff)


@dataclass(frozen=True)
class Device:
    """What the tool reads before writing anything."""
    security: Security
    partitions: list[Partition]
    booting: Slot
    slots: dict[Slot, AppImage | None]

    def stock_slot(self):
        """Where Habity's firmware waits to be booted again: app0 when an update put it
        there, factory otherwise."""
        return next((s for s in (Slot.APP0, Slot.FACTORY) if self.slots.get(s) and self.slots[s].is_stock), None)

    def layout_refusals(self):
        """Why writing otadata would not be safe; empty when it is."""
        refusals = []
        if self.security.secure_boot or self.security.flash_encryption:
            refusals.append('secure boot or flash encryption is on: this device only boots images '
                            'signed by Habity.')
        by_kind = {(p.type, p.subtype): p for p in self.partitions}
        for expected in _EXPECTED_PARTITIONS:
            got = by_kind.get((expected.type, expected.subtype))
            if not got or (got.offset, got.size) != (expected.offset, expected.size):
                refusals.append(f'the partition table is not the one this tool knows: expected {expected.label} '
                                f'at 0x{expected.offset:x} ({expected.size // 1024} KB), found '
                                f'{f"0x{got.offset:x} ({got.size // 1024} KB)" if got else "none"}.')
        return refusals

    def install_refusals(self):
        """Why putting an image in app1 would not be safe; empty when it is."""
        refusals = self.layout_refusals()
        app1 = self.slots.get(Slot.APP1)
        if app1 and app1.is_stock and self.booting == Slot.APP1:
            refusals.append(f"Habity's firmware ({app1}) runs from app1, where cute-display goes: "
                            'installing would erase it.')
        if not self.stock_slot():
            refusals.append("Habity's firmware is in neither factory nor app0: there would be no way back.")
        return refusals

    def boot_refusals(self, slot):
        """Why the device should not be made to start `slot`; empty when it may."""
        return self.layout_refusals() + ([] if self.slots.get(slot) else [f'{slot} is empty.'])

    def install_warnings(self):
        stock = self.stock_slot()
        held = self.slots[stock] if stock else None
        if held and held.version not in TESTED_STOCK_VERSIONS:
            return [f'{held} has not been tried with cute-display (tried: '
                    f'{", ".join(sorted(TESTED_STOCK_VERSIONS))}).']
        return []
