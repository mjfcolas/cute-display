"""The clock on the USB cable, as the setup's screens use it: each step one call, which
may take from seconds to minutes, and fails with `Failed` and a message for people."""
import time
from urllib.error import URLError

from esptool.cmds import FatalError
from serial import SerialException

from .. import releases, usb
from ..card import console
from ..flash import device
from ..flash.layout import image_refusals
from . import card
from .answers import CARD_FILES, Answers

# Cute Display answers on its console a few seconds after a restart, later with a cold
# SD card.
STARTING_TIMEOUT_S = 40


class Failed(Exception):
    """A step could not be done; the message says why."""


def _said(error):
    """`error` as people read it; None for one no step expects."""
    if isinstance(error, (usb.NoDevice, Failed)):
        return str(error)
    if isinstance(error, device.Refused):
        return f'Cannot install: {error}'
    if isinstance(error, (FatalError, SerialException)):
        return usb.explain(error)
    if isinstance(error, console.ConsoleError):
        return f'SD card: {error}'
    if isinstance(error, releases.ReleaseError):
        return f'GitHub: {error}.'
    if isinstance(error, (URLError, TimeoutError)):
        return f'{getattr(error, "reason", error)}: is this computer online?'
    return None


def _step(call):
    """`call()`, its expected failures turned into `Failed`."""
    try:
        return call()
    except Exception as error:
        said = _said(error)
        if said is None:
            raise
        raise Failed(said) from None


class UsbClock:
    def read(self):
        def read():
            with device.connect() as esp:
                unit = device.read_device(esp)
                device.restart(esp)
            return unit
        return _step(read)

    def back_up(self, folder):
        """The whole flash into a file in `folder`; its path."""
        path = device.backup_path(folder)

        def back_up():
            with device.connect() as esp:
                device.save_flash(esp, path)
                device.restart(esp)
            return path
        return _step(back_up)

    def latest(self):
        """The latest release's image, checked: whole, and a Cute Display image that fits."""
        def latest():
            contents = releases.latest_image(report=lambda _: None)[1]
            refusals = image_refusals(contents)
            if refusals:
                raise Failed(f'The latest release cannot be installed: {" ".join(refusals)}')
            return contents
        return _step(latest)

    def install(self, contents):
        def install():
            with device.connect() as esp:
                device.install_checked(esp, device.read_device(esp), contents, lambda _: True)
        return _step(install)

    def start(self, slot):
        def start():
            with device.connect() as esp:
                device.boot_into(esp, slot)
        return _step(start)

    def wait_for_cute_display(self):
        """Until the console of the Cute Display that just started answers."""
        deadline = time.monotonic() + STARTING_TIMEOUT_S
        while True:
            try:
                with usb.open_link() as link:
                    console.entries(link)
                return
            except (usb.NoDevice, SerialException, console.ConsoleError) as error:
                if time.monotonic() > deadline:
                    raise Failed(_said(error)) from None
                time.sleep(1)

    def read_card(self):
        def read_card():
            with usb.open_link() as link:
                return Answers.from_card(card.read_texts(link, CARD_FILES), card.habity_ringtones_missing(link))
        return _step(read_card)

    def write_card(self, files, copies):
        def write_card():
            with usb.open_link() as link:
                card.write(link, files, copies)
        return _step(write_card)

    def restart(self):
        def restart():
            with device.connect() as esp:
                device.restart(esp)
        return _step(restart)
