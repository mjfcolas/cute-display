"""The device's flash through esptool: read what layout.py judges, write what it allows."""
import os
import time

from esptool.cmds import attach_flash, detect_chip, read_flash, reset_chip, run_stub, verify_flash, write_flash
from esptool.logger import log

from .. import usb
from .layout import (APP_HEADER_SIZE, FLASH_SIZE, OTADATA, OTADATA_SIZE, PARTITION_TABLE, PARTITION_TABLE_SIZE,
                     Device, Security, Slot, app_image, booting_slot, image_refusals, otadata_booting,
                     partitions)


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


def backup_path(folder):
    return os.path.join(os.path.abspath(folder), f'habity-flash-{time.strftime("%Y%m%d-%H%M%S")}.bin')


class Refused(Exception):
    """Why an install did not happen; nothing was written."""

    def __init__(self, refusals):
        super().__init__(' '.join(refusals))
        self.refusals = refusals


def install_checked(esp, unit, contents, go_ahead):
    """`contents` installed on `unit`, the device as just read, once the image and the
    device pass their checks and `go_ahead(placement)` agrees; the placement. Raises
    `Refused`, the device restarted, when not."""
    refusals = image_refusals(contents) + unit.install_refusals()
    placement = None if refusals else unit.placement()
    if not refusals and not go_ahead(placement):
        refusals = ['not confirmed; nothing was written.']
    if refusals:
        restart(esp)
        raise Refused(refusals)
    install(esp, placement, unit.habity(besides=placement.slot), contents)
    return placement


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
