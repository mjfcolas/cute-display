"""The device on USB, running the test image (`just fw e2e`): its console, which also
carries its log, and its restart."""
import time

from cute_display_link import card, clock, usb
from cute_display_link.console import ConsoleError, KeepingLog

from .log import Log

OWN_DIRECTORY = 'cute-display-e2e'
CLOCKS_WIFI_CONF = 'cute-display/wifi.conf'
# From the reset until the console answers: about 2.5 s.
STARTING_TIMEOUT_S = 20


class NotTheTestImage(Exception):
    pass


class Device:
    """`log` holds what came in since it was last replaced."""

    def __init__(self, link):
        self.log = Log()
        self.link = KeepingLog(link)

    def restart(self):
        self.keep_log()
        usb.restart(self.link.link)
        deadline = time.monotonic() + STARTING_TIMEOUT_S
        while True:
            try:
                clock.read(self.link)
                return
            except ConsoleError:
                if time.monotonic() > deadline:
                    raise ConsoleError(f'the device did not answer {STARTING_TIMEOUT_S} s after its restart') from None

    def empty_own_directory_but_the_clocks_wifi(self):
        try:
            for path in self._files_in(OWN_DIRECTORY):
                card.remove(self.link, path)
            card.copy(self.link, CLOCKS_WIFI_CONF, f'{OWN_DIRECTORY}/wifi.conf')
        except ConsoleError as refused:
            if 'may be written' in str(refused):
                raise NotTheTestImage(f'the device runs another image ({refused}): flash the test image, `just fw e2e`') from None
            raise

    def put(self, contents_by_path_in_own_directory):
        for path, contents in contents_by_path_in_own_directory.items():
            card.write_file(self.link, f'{OWN_DIRECTORY}/{path}', contents)

    def _files_in(self, directory):
        try:
            entries = card.entries(self.link, directory)
        except ConsoleError as missing:
            if str(missing) == 'no such directory':
                return []
            raise
        files = []
        for entry in entries:
            path = f'{directory}/{entry.name}'
            files += self._files_in(path) if entry.is_directory else [path]
        return files

    def keep_log(self):
        for line in self.link.take_log():
            self.log.add(line)

    def close(self):
        self.link.link.close()
