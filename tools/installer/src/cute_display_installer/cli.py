"""The `cute-display` command."""
import argparse
import os
import sys
import time
from urllib.error import URLError

from esptool.cmds import FatalError
from serial import SerialException

from . import rtc, usb
from .card import console
from .card.copy import pull
from .config import airports, radar
from .flash import device
from .flash.layout import APP_HEADER_SIZE, FLASH_SIZE, Slot, app_image, image_refusals

RTC_LOG_TIMEOUT_S = 20


def describe(unit):
    for slot in Slot:
        image = unit.slots[slot] or 'empty'
        print(f'  {slot.label:<8} {image}{"   <- boots" if slot == unit.booting else ""}')
    for warning in unit.install_warnings():
        print(f'Warning: {warning}')


def refuse(esp, doing, refusals):
    device.restart(esp)
    sys.exit('\n'.join(f'Cannot {doing}: {r}' for r in refusals))


def check():
    print('Connecting to the device...')
    with device.connect() as esp:
        unit = device.read_device(esp)
        device.restart(esp)
    describe(unit)
    refusals = unit.install_refusals()
    for refusal in refusals:
        print(f'Cannot install: {refusal}')
    if refusals:
        sys.exit(1)
    print('This device can take cute-display.')


def backup(directory):
    os.makedirs(directory, exist_ok=True)
    path = os.path.join(directory, f'habity-flash-{time.strftime("%Y%m%d-%H%M%S")}.bin')
    print('Connecting to the device...')
    with device.connect() as esp:
        print(f'Reading its {FLASH_SIZE // 2**20} MB of flash; this takes a few minutes...')
        device.save_flash(esp, path)
        device.restart(esp)
    print(f'The whole flash is in {path}.')
    print('It holds your Wi-Fi password: keep it to yourself, never attach it to an issue.')


def install(image):
    with open(image, 'rb') as f:
        contents = f.read()
    refusals = image_refusals(contents)
    if refusals:
        sys.exit('\n'.join(f'Cannot install {image}: {r}' for r in refusals))
    new = app_image(contents[:APP_HEADER_SIZE])
    print('Connecting to the device...')
    with device.connect() as esp:
        unit = device.read_device(esp)
        describe(unit)
        refusals = unit.install_refusals()
        if refusals:
            refuse(esp, 'install', refusals)
        print(f'Writing {new} into app1; this takes a minute...')
        device.write_app1(esp, contents)
        device.boot_into(esp, Slot.APP1)
    print(f"Installed. The device now starts {new}; `boot app0` or `boot factory` takes it back to "
          "Habity's firmware.")


def boot(slot):
    print('Connecting to the device...')
    with device.connect() as esp:
        unit = device.read_device(esp)
        describe(unit)
        refusals = unit.boot_refusals(slot)
        if refusals:
            refuse(esp, f'boot {slot}', refusals)
        device.boot_into(esp, slot)
    print(f'Done. The device now starts {unit.slots[slot]}, from {slot}.')


def card_ls(directory):
    with usb.open_link() as link:
        for entry in console.entries(link, directory):
            print(f'{"d" if entry.is_directory else "-"} {entry.size:>10} {entry.name}')


def card_get(path, destination):
    with usb.open_link() as link:
        contents = console.read_file(link, path)
    if destination:
        with open(destination, 'wb') as f:
            f.write(contents)
        print(f'{path} -> {destination} ({len(contents)} bytes)')
    else:
        sys.stdout.buffer.write(contents)


def card_put(source, path):
    with open(source, 'rb') as f:
        contents = f.read()
    with usb.open_link() as link:
        console.write_file(link, path, contents)
    print(f'{source} -> {path} ({len(contents)} bytes)')


def card_rm(path):
    with usb.open_link() as link:
        console.remove(link, path)
    print(f'removed {path}')


def card_pull(directory, destination):
    with usb.open_link() as link:
        pull(link, directory, destination)


def radar_airports(at):
    rows = airports.ourairports(report=lambda message: print(message, file=sys.stderr))
    if at:
        print('\n'.join(airports.around(*at, rows)))
        return
    with usb.open_link() as link:
        lines, (latitude, longitude) = radar.put_airports(link, rows)
    print(f'{len(lines)} airports within {airports.RADIUS_KM} km of {latitude}, {longitude}')


def rtc_registers(destination):
    print('Restarting the device...')
    with device.connect() as esp:
        device.restart(esp)
    deadline = time.monotonic() + RTC_LOG_TIMEOUT_S
    while time.monotonic() < deadline:
        try:
            link = usb.open_link()
        except (SerialException, usb.NoDevice):
            time.sleep(0.2)
            continue
        with link:
            while time.monotonic() < deadline:
                registers = rtc.registers_in(link.readline().decode(errors='replace'))
                if registers is not None:
                    with open(destination, 'wb') as f:
                        f.write(registers)
                    print(f'{len(registers)} registers -> {destination}')
                    print('\n'.join(rtc.describe(registers)))
                    return
    sys.exit('The registers never showed in the log; is the hardware test image running?')


def explain(error):
    """What to do about a port that would not open, or a device that stopped answering."""
    text = str(error)
    if 'busy' in text or 'Access is denied' in text:
        return 'The port is in use: close any serial monitor on it (another terminal, an IDE) and try again.'
    if 'Permission denied' in text:
        return ('This user may not open the port: on Linux, join the group that owns it '
                '(dialout or uucp), then log in again.')
    return f'The device stopped answering ({text}). Unplug it, plug it back, and try again.'


def parser():
    top = argparse.ArgumentParser(
        prog='cute-display',
        description="Install cute-display on a Habity bedside clock, and go back to Habity's firmware.",
    )
    commands = top.add_subparsers(required=True, metavar='command')

    def command(group, name, run, help_text):
        sub = group.add_parser(name, help=help_text)
        sub.set_defaults(run=run)
        return sub

    command(commands, 'check', check, 'what the device holds, and whether it can take cute-display')
    command(commands, 'backup', backup, 'the whole flash into a file (holds the Wi-Fi password)') \
        .add_argument('directory', nargs='?', default='.')
    command(commands, 'install', install, 'cute-display into app1, and boot it').add_argument('image')
    command(commands, 'boot', boot, 'boot the firmware in one slot') \
        .add_argument('slot', type=Slot.named, choices=list(Slot))

    card = commands.add_parser('card', help="the device's SD card, cute-display running") \
        .add_subparsers(required=True, metavar='action')
    command(card, 'ls', card_ls, 'list a directory').add_argument('directory', nargs='?', default='')
    get_parser = command(card, 'get', card_get, 'copy a file off the card (to the terminal without a destination)')
    get_parser.add_argument('path')
    get_parser.add_argument('destination', nargs='?')
    put_parser = command(card, 'put', card_put, 'copy a file onto the card, under cute-display/')
    put_parser.add_argument('source')
    put_parser.add_argument('path')
    command(card, 'rm', card_rm, 'remove a file under cute-display/').add_argument('path')
    pull_parser = command(card, 'pull', card_pull, 'copy a directory of the card, resuming where it stopped')
    pull_parser.add_argument('directory')
    pull_parser.add_argument('destination')

    command(commands, 'radar-airports', radar_airports,
            "the airports around radar.conf's place onto the card") \
        .add_argument('--at', nargs=2, type=float, metavar=('LATITUDE', 'LONGITUDE'),
                      help='only print those around this place')
    command(commands, 'rtc-registers', rtc_registers,
            "the DS3231's registers into a file, the hardware test image running").add_argument('destination')
    return top


def main():
    arguments = vars(parser().parse_args())
    run = arguments.pop('run')
    try:
        run(**arguments)
    except usb.NoDevice as error:
        sys.exit(str(error))
    except (console.ConsoleError, radar.NoPlace) as error:
        sys.exit(f'SD card: {error}')
    except rtc.NoRegisters as error:
        sys.exit(str(error))
    except URLError as error:
        sys.exit(f'OurAirports: {error.reason}. Is this computer online?')
    except FileNotFoundError as error:
        sys.exit(f'{error.filename}: no such file.')
    except (FatalError, SerialException) as error:
        sys.exit(explain(error))
