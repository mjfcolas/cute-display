#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = ["esptool>=5.1", "pyserial"]
# ///
"""Install cute-display on a Habity bedside clock, and go back to Habity's firmware.

Nothing is written before the device has been checked. The device is found by its USB
name; CUTE_DISPLAY_PORT overrides it. flash_layout.py holds the rules.
"""
import argparse
import os
import sys
import time

from esptool.logger import log
from esptool.cmds import (FatalError, SerialException, attach_flash, detect_chip, read_flash, reset_chip,
                          run_stub, write_flash)
from serial.tools import list_ports

from flash_layout import (APP_HEADER_SIZE, FLASH_SIZE, OTADATA, OTADATA_SIZE, PARTITION_TABLE, PARTITION_TABLE_SIZE,
                          Device, Security, Slot, app_image, booting_slot, image_refusals, otadata_booting,
                          partitions)

ESPRESSIF_USB = (0x303a, 0x1001)


def port():
    chosen = os.environ.get('CUTE_DISPLAY_PORT')
    if chosen:
        return chosen
    found = [p.device for p in list_ports.comports() if (p.vid, p.pid) == ESPRESSIF_USB]
    if not found:
        sys.exit('No Habity found on USB. Is the cable plugged in, and is it a data cable?')
    if len(found) > 1:
        sys.exit(f'Several devices found ({", ".join(found)}): choose one with CUTE_DISPLAY_PORT.')
    return found[0]


def connect():
    print('Connecting to the device...')
    log.set_verbosity('silent')
    esp = run_stub(detect_chip(port()))
    attach_flash(esp)
    return esp


def read_bytes(esp, address, size):
    return read_flash(esp, address, size, None, no_progress=True)


def read_device(esp):
    return Device(
        security=Security(secure_boot=bool(esp.get_secure_boot_enabled()),
                          flash_encryption=bool(esp.get_flash_encryption_enabled())),
        partitions=partitions(read_bytes(esp, PARTITION_TABLE, PARTITION_TABLE_SIZE)),
        booting=booting_slot(read_bytes(esp, OTADATA, OTADATA_SIZE)),
        slots={slot: app_image(read_bytes(esp, slot.offset, APP_HEADER_SIZE)) for slot in Slot},
    )


def describe(device):
    for slot in Slot:
        held = device.slots[slot] or 'empty'
        print(f'  {slot.label:<8} {held}{"   <- boots" if slot == device.booting else ""}')
    for warning in device.install_warnings():
        print(f'Warning: {warning}')


def refuse(esp, doing, refusals):
    reset_chip(esp, 'hard-reset')
    sys.exit('\n'.join(f'Cannot {doing}: {r}' for r in refusals))


def boot_into(esp, slot):
    write_flash(esp, [(OTADATA, otadata_booting(slot))], no_progress=True)
    reset_chip(esp, 'hard-reset')


def check():
    with connect() as esp:
        device = read_device(esp)
        reset_chip(esp, 'hard-reset')
    describe(device)
    refusals = device.install_refusals()
    for refusal in refusals:
        print(f'Cannot install: {refusal}')
    if refusals:
        sys.exit(1)
    print('This device can take cute-display.')


def backup(directory='.'):
    os.makedirs(directory, exist_ok=True)
    path = os.path.join(directory, f'habity-flash-{time.strftime("%Y%m%d-%H%M%S")}.bin')
    with connect() as esp:
        print(f'Reading its {FLASH_SIZE // 2**20} MB of flash; this takes a few minutes...')
        read_flash(esp, 0, FLASH_SIZE, path)
        reset_chip(esp, 'hard-reset')
    print(f'The whole flash is in {path}.')
    print('It holds your Wi-Fi password: keep it to yourself, never attach it to an issue.')


def install(image):
    with open(image, 'rb') as f:
        contents = f.read()
    refusals = image_refusals(contents)
    if refusals:
        sys.exit('\n'.join(f'Cannot install {image}: {r}' for r in refusals))
    held = app_image(contents[:APP_HEADER_SIZE])
    with connect() as esp:
        device = read_device(esp)
        describe(device)
        refusals = device.install_refusals()
        if refusals:
            refuse(esp, 'install', refusals)
        print(f'Writing {held} into app1; this takes a minute...')
        write_flash(esp, [(Slot.APP1.offset, contents)])
        boot_into(esp, Slot.APP1)
    print(f"Installed. The device now starts {held}; `boot app0` or `boot factory` takes it back to Habity's firmware.")


def boot(slot):
    with connect() as esp:
        device = read_device(esp)
        describe(device)
        refusals = device.boot_refusals(slot)
        if refusals:
            refuse(esp, f'boot {slot}', refusals)
        boot_into(esp, slot)
    print(f'Done. The device now starts {device.slots[slot]}, from {slot}.')


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    commands = parser.add_subparsers(dest='command', required=True)
    commands.add_parser('check', help='what the device holds, and whether it can take cute-display') \
        .set_defaults(run=check)
    backup_parser = commands.add_parser('backup', help='the whole flash into a file (holds the Wi-Fi password)')
    backup_parser.add_argument('directory', nargs='?', default='.')
    backup_parser.set_defaults(run=backup)
    install_parser = commands.add_parser('install', help='cute-display into app1, and boot it')
    install_parser.add_argument('image')
    install_parser.set_defaults(run=install)
    boot_parser = commands.add_parser('boot', help='boot the firmware in one slot')
    boot_parser.add_argument('slot', type=Slot.named, choices=list(Slot))
    boot_parser.set_defaults(run=boot)
    arguments = vars(parser.parse_args())
    del arguments['command']
    run = arguments.pop('run')
    try:
        run(**arguments)
    except (FatalError, SerialException) as e:
        sys.exit(f'The device stopped answering: {e}\nUnplug it, plug it back, and run the same command again.')


if __name__ == '__main__':
    main()
