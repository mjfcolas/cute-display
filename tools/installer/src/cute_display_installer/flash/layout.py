"""The Habity's flash: its partitions, which app the bootloader starts, what each app slot
holds, where Cute Display can go and how to come back out.

Everything here works on bytes read from the flash; device.py reads and writes them.
Only otadata and the OTA slot Cute Display goes into are ever written: factory keeps the
Habity firmware the clock was shipped with, and going back to Habity is a matter of
otadata alone.
"""
import struct
import zlib
from dataclasses import dataclass
from enum import Enum

from .. import versions

FLASH_SIZE = 0x1000000
PARTITION_TABLE = 0x8000
PARTITION_TABLE_SIZE = 0xc00
OTADATA = 0x19000
OTADATA_SIZE = 0x2000
OTADATA_SECTOR = 0x1000
APP_SIZE = 0x400000

STOCK_PROJECT = 'habity'
OUR_PROJECT = 'cute-display'
OUR_NAME = 'Cute Display'

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

PARTITION_MAGIC = b'\xaa\x50'
PARTITION_ENTRY_SIZE = 32
PARTITION_LABEL = slice(12, 28)

_IMAGE_MAGIC = 0xe9
# esp_app_desc_t sits right after the image header and its first segment's header.
APP_DESC_OFFSET = 0x20
_APP_DESC_SIZE = 0x100
_APP_DESC_MAGIC = 0xabcd5432
APP_DESC_VERSION = slice(16, 48)
_APP_DESC_PROJECT = slice(48, 80)
APP_HEADER_SIZE = APP_DESC_OFFSET + _APP_DESC_SIZE

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

    @property
    def is_ours(self):
        return self.project == OUR_PROJECT

    @property
    def release(self):
        return versions.release(self.version)

    def __str__(self):
        name = {STOCK_PROJECT: 'Habity', OUR_PROJECT: OUR_NAME}.get(self.project, self.project)
        return f'{name} {self.version}'


_UNDESCRIBED = AppImage(project='unknown', version='unknown')


@dataclass(frozen=True)
class Security:
    secure_boot: bool
    flash_encryption: bool


def partitions(table):
    found = []
    for at in range(0, len(table) - PARTITION_ENTRY_SIZE + 1, PARTITION_ENTRY_SIZE):
        entry = table[at:at + PARTITION_ENTRY_SIZE]
        if entry[:2] != PARTITION_MAGIC:
            break
        offset, size = struct.unpack_from('<II', entry, 4)
        found.append(Partition(_c_string(entry[PARTITION_LABEL]), entry[2], entry[3], offset, size))
    return found


def app_image(header):
    """What an app slot holds, from its first APP_HEADER_SIZE bytes; None when it holds
    no image."""
    if len(header) < APP_HEADER_SIZE or header[0] != _IMAGE_MAGIC:
        return None
    desc = header[APP_DESC_OFFSET:]
    if struct.unpack_from('<I', desc)[0] != _APP_DESC_MAGIC:
        return _UNDESCRIBED
    return AppImage(project=_c_string(desc[_APP_DESC_PROJECT]), version=_c_string(desc[APP_DESC_VERSION]))


def image_refusals(contents):
    """Why a file may not be installed; empty when it may."""
    held = app_image(contents[:APP_HEADER_SIZE])
    if held is None:
        return ['it is not an app image.']
    refusals = []
    if held == _UNDESCRIBED:
        refusals.append('the image does not say what it is.')
    elif held.project != OUR_PROJECT:
        refusals.append(f'it is not a Cute Display image: it says {held}.')
    if len(contents) > APP_SIZE:
        refusals.append(f'it is larger than a slot ({len(contents) // 1024} KB, a slot holds {APP_SIZE // 1024}).')
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


class Target(Enum):
    """What `cute-display boot` is told to start: a firmware by its name, or a slot."""
    HABITY = 'habity'
    FACTORY = 'factory'
    CUTE_DISPLAY = 'cute-display'
    APP0 = 'app0'
    APP1 = 'app1'

    def __str__(self):
        return self.value


@dataclass(frozen=True)
class Placement:
    """Where an install writes Cute Display, and the Habity firmware it erases there, if any."""
    slot: Slot
    erases: AppImage | None = None

    def __str__(self):
        if self.erases:
            return f'into {self.slot}, erasing {self.erases} there, the older of the two'
        return f'into {self.slot}'


@dataclass(frozen=True)
class Device:
    """What the tool reads before writing anything."""
    security: Security
    partitions: list[Partition]
    booting: Slot
    slots: dict[Slot, AppImage | None]

    def _holding(self, found, among=OTA_SLOTS_BY_SEQUENCE):
        return [slot for slot in among if self.slots.get(slot) and found(self.slots[slot])]

    def slot_lines(self):
        """Each slot, what it holds, and which one boots, a line each."""
        return [f'{slot.label:<8} {self.slots[slot] or "empty"}{"   <- boots" if slot == self.booting else ""}'
                for slot in Slot]

    def ours(self):
        """The OTA slot Cute Display is in; the booting one if both hold it."""
        slots = self._holding(lambda image: image.is_ours)
        return self.booting if self.booting in slots else next(iter(slots), None)

    def habity(self, besides=None):
        """Where the newest Habity firmware is, `besides` left out: an OTA slot when it
        updated itself, factory otherwise; None when there is none, or when two cannot be
        told apart by their versions."""
        updated = self._holding(lambda image: image.is_stock, [s for s in OTA_SLOTS_BY_SEQUENCE if s != besides])
        if len(updated) == 2 and any(self.slots[s].release is None for s in updated):
            return None
        if updated:
            return max(updated, key=lambda s: self.slots[s].release or ())
        return Slot.FACTORY if self.slots.get(Slot.FACTORY) and self.slots[Slot.FACTORY].is_stock else None

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

    def placement(self):
        """Where Cute Display goes: the slot it is already in, else a slot without Habity's
        firmware, else the one with the older Habity firmware. None when neither will do."""
        if self.ours():
            return Placement(self.ours())
        free = [slot for slot in OTA_SLOTS_BY_SEQUENCE if not (self.slots.get(slot) and self.slots[slot].is_stock)]
        if free:
            return Placement(free[0])
        releases = {slot: self.slots[slot].release for slot in OTA_SLOTS_BY_SEQUENCE}
        if None in releases.values():
            return None
        older = min(OTA_SLOTS_BY_SEQUENCE, key=releases.get)
        return Placement(older, erases=self.slots[older])

    def install_refusals(self):
        """Why installing would not be safe; empty when it is."""
        refusals = self.layout_refusals()
        placement = self.placement()
        if placement is None:
            refusals.append(f"app0 and app1 both hold Habity's firmware, {self.slots[Slot.APP0]} and "
                            f"{self.slots[Slot.APP1]}, and one of the versions does not read as numbers: "
                            'which one is older cannot be told.')
        elif self.habity(besides=placement.slot) is None:
            refusals.append("no Habity firmware would be left beside Cute Display: there would be no way back.")
        return refusals

    def slot_for(self, target):
        """The slot `target` names on this device; None when it names none."""
        return {
            Target.HABITY: self.habity(),
            Target.FACTORY: Slot.FACTORY,
            Target.CUTE_DISPLAY: self.ours(),
            Target.APP0: Slot.APP0,
            Target.APP1: Slot.APP1,
        }[target]

    def boot_refusals(self, target):
        """Why the device should not be made to start `target`; empty when it may."""
        refusals = self.layout_refusals()
        slot = self.slot_for(target)
        if slot is None:
            refusals.append(f'no {target} to start here: `cute-display check` shows what each slot holds.')
        elif not self.slots.get(slot):
            refusals.append(f'{slot} is empty.')
        return refusals
