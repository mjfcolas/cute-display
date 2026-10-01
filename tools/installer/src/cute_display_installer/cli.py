"""The `cute-display` command."""
import argparse
import os
import sys
import time
from urllib.error import HTTPError, URLError

from esptool.cmds import FatalError
from serial import SerialException

from cute_display_link import card, usb
from cute_display_link.console import ConsoleError

from . import releases, rtc, updating, versions, web
from .card.copy import pull
from .config import airports, general, places, radar
from .flash import device
from .flash.layout import APP_HEADER_SIZE, FLASH_SIZE, Slot, Target, app_image, image_refusals
from .setup.clock import UsbClock
from .setup.app import SetupApp

RTC_LOG_TIMEOUT_S = 20


def describe(unit):
    for line in unit.slot_lines():
        print(f'  {line}')


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
    print(f'This device can take Cute Display, {unit.placement()}.')


def backup(directory):
    os.makedirs(directory, exist_ok=True)
    path = device.backup_path(directory)
    print('Connecting to the device...')
    with device.connect() as esp:
        print(f'Reading its {FLASH_SIZE // 2**20} MB of flash; this takes a few minutes...')
        device.save_flash(esp, path)
        device.restart(esp)
    print(f'The whole flash is in {path}.')
    print('It holds your Wi-Fi password: keep it to yourself, never attach it to an issue.')


def confirmed(question):
    return input(f'{question} [y/N] ').strip().lower() in ('y', 'yes')


def install(image, yes):
    updating_entries, installer_refusals = (), []
    if image:
        with open(image, 'rb') as f:
            contents = f.read()
    else:
        release = releases.latest_release()
        image, contents, updating_entries = release.name, release.image, release.updating_entries
        installer_refusals = release.installer_refusals(versions.this_installer_version())
    refusals = image_refusals(contents) + installer_refusals
    if refusals:
        sys.exit('\n'.join(f'Cannot install {image}: {r}' for r in refusals))
    new = app_image(contents[:APP_HEADER_SIZE])

    def go_ahead(placement):
        ours = unit.ours()
        to_know = () if ours is None else updating.to_know(updating_entries, unit.slots[ours], new)
        for entry in to_know:
            print(f'\nBefore updating to {entry.version}:\n{entry.text}\n')
        if to_know and not (yes or confirmed(f'Update {unit.slots[ours]} to {new}?')):
            return False
        if placement.erases and not (yes or confirmed(
                f'Both slots hold Habity\'s firmware: {placement.erases} in {placement.slot} is erased for '
                f'Cute Display, {unit.slots[unit.habity(besides=placement.slot)]} stays. Go on?')):
            return False
        print(f'Writing {new} {placement}; this takes a minute...')
        return True

    print('Connecting to the device...')
    with device.connect() as esp:
        unit = device.read_device(esp)
        describe(unit)
        try:
            device.install_checked(esp, unit, contents, go_ahead)
        except device.Refused as refused:
            sys.exit('\n'.join(f'Cannot install: {r}' for r in refused.refusals))
    print(f"Installed. The device now starts {new}; `boot habity` takes it back to Habity's firmware.")


def boot(target):
    print('Connecting to the device...')
    with device.connect() as esp:
        unit = device.read_device(esp)
        describe(unit)
        refusals = unit.boot_refusals(target)
        if refusals:
            refuse(esp, f'boot {target}', refusals)
        slot = unit.slot_for(target)
        device.boot_into(esp, slot)
    print(f'Done. The device now starts {unit.slots[slot]}, from {slot}.')


def card_ls(directory):
    with usb.open_link() as link:
        for entry in card.entries(link, directory):
            print(f'{"d" if entry.is_directory else "-"} {entry.size:>10} {entry.name}')


def card_get(path, destination):
    with usb.open_link() as link:
        contents = card.read_file(link, path)
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
        card.write_file(link, path, contents)
    print(f'{source} -> {path} ({len(contents)} bytes)')


def card_rm(path):
    with usb.open_link() as link:
        card.remove(link, path)
    print(f'removed {path}')


def card_pull(directory, destination):
    with usb.open_link() as link:
        pull(link, directory, destination)


def radar_airports(at):
    rows = airports.ourairports(report=lambda message: print(message, file=sys.stderr))
    if at:
        print('\n'.join(airport.line for airport in airports.around(*at, rows)))
        return
    with usb.open_link() as link:
        found, place = radar.put_airports(link, rows)
    print(f'{len(found)} airports within {airports.RADIUS_KM} km of {place.name} ({place.latitude}, {place.longitude})')


def setup():
    def airports_around(place):
        return airports.around(place.latitude, place.longitude, airports.ourairports(report=lambda _: None))

    SetupApp(UsbClock(), lambda name: places.search(name, web.fetch), airports_around).run()


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


def parser():
    top = argparse.ArgumentParser(
        prog='cute-display',
        description="Install Cute Display on a Habity bedside clock, and go back to Habity's firmware.",
    )
    commands = top.add_subparsers(required=True, metavar='command')

    def command(group, name, run, help_text):
        sub = group.add_parser(name, help=help_text)
        sub.set_defaults(run=run)
        return sub

    command(commands, 'check', check, 'what the device holds, and whether it can take Cute Display')
    command(commands, 'backup', backup, 'the whole flash into a file (holds the Wi-Fi password)') \
        .add_argument('directory', nargs='?', default='.')
    install_parser = command(commands, 'install', install, 'Cute Display into app0 or app1, and boot it')
    install_parser.add_argument('image', nargs='?', help="an image file; the latest release's without one")
    install_parser.add_argument('--yes', action='store_true',
                                help="erase the older Habity firmware without asking, when both slots hold one")
    command(commands, 'boot', boot, "start Habity's newest firmware, the factory one, Cute Display, or a slot") \
        .add_argument('target', type=Target, choices=list(Target))

    card_commands = commands.add_parser('card', help="the device's SD card, Cute Display running") \
        .add_subparsers(required=True, metavar='action')
    command(card_commands, 'ls', card_ls, 'list a directory').add_argument('directory', nargs='?', default='')
    get_parser = command(card_commands, 'get', card_get, 'copy a file off the card (to the terminal without a destination)')
    get_parser.add_argument('path')
    get_parser.add_argument('destination', nargs='?')
    put_parser = command(card_commands, 'put', card_put, 'copy a file onto the card, under cute-display/')
    put_parser.add_argument('source')
    put_parser.add_argument('path')
    command(card_commands, 'rm', card_rm, 'remove a file under cute-display/').add_argument('path')
    pull_parser = command(card_commands, 'pull', card_pull, 'copy a directory of the card, resuming where it stopped')
    pull_parser.add_argument('directory')
    pull_parser.add_argument('destination')

    command(commands, 'setup', setup, 'step by step: back up the clock, install or update Cute Display, set it up')
    command(commands, 'radar-airports', radar_airports,
            "the airports around general.conf's place onto the card") \
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
    except (ConsoleError, general.NoPlace) as error:
        sys.exit(f'SD card: {error}')
    except rtc.NoRegisters as error:
        sys.exit(str(error))
    except releases.ReleaseError as error:
        sys.exit(f'GitHub: {error}.')
    except HTTPError as error:
        sys.exit(f'{error.url} answered {error.code} {error.reason}.')
    except URLError as error:
        sys.exit(f'{error.reason}: is this computer online?')
    except FileNotFoundError as error:
        sys.exit(f'{error.filename}: no such file.')
    except (FatalError, SerialException) as error:
        sys.exit(usb.explain(error))
