"""The device's flash through esptool: read what layout.py judges, write what it allows."""
import os

from esptool.cmds import attach_flash, detect_chip, read_flash, reset_chip, run_stub, verify_flash, write_flash
from esptool.logger import log

from .. import usb
from .layout import (APP_HEADER_SIZE, FLASH_SIZE, OTADATA, OTADATA_SIZE, PARTITION_TABLE, PARTITION_TABLE_SIZE,
                     Device, Security, Slot, app_image, booting_slot, otadata_booting, partitions)


def connect():
    """The device in its ROM bootloader, esptool's stub running; a context manager."""
    log.set_verbosity('silent')
    esp = run_stub(detect_chip(usb.port()))
    attach_flash(esp)
    return esp


def restart(esp):
    reset_chip(esp, 'hard-reset')


def _read_bytes(esp, address, size):
    return read_flash(esp, address, size, None, no_progress=True)


def read_device(esp):
    return Device(
        security=Security(secure_boot=bool(esp.get_secure_boot_enabled()),
                          flash_encryption=bool(esp.get_flash_encryption_enabled())),
        partitions=partitions(_read_bytes(esp, PARTITION_TABLE, PARTITION_TABLE_SIZE)),
        booting=booting_slot(_read_bytes(esp, OTADATA, OTADATA_SIZE)),
        slots={slot: app_image(_read_bytes(esp, slot.offset, APP_HEADER_SIZE)) for slot in Slot},
    )


def save_flash(esp, path):
    """The whole flash into `path`, which only appears once checked against the flash."""
    part = path + '.part'
    try:
        read_flash(esp, 0, FLASH_SIZE, part)
        verify_flash(esp, [(0, part)])
    except BaseException:
        if os.path.exists(part):
            os.remove(part)
        raise
    os.replace(part, path)


def install(esp, placement, way_back, contents):
    """Writes `contents` into the placement's slot and makes the bootloader start it. Until
    it is written whole, otadata points at `way_back`: cut short anywhere, the clock
    starts that."""
    _point_otadata_at(esp, way_back)
    write_flash(esp, [(placement.slot.offset, contents)])
    boot_into(esp, placement.slot)


def _point_otadata_at(esp, slot):
    write_flash(esp, [(OTADATA, otadata_booting(slot))], no_progress=True)


def boot_into(esp, slot):
    _point_otadata_at(esp, slot)
    restart(esp)
