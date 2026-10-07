"""`bench`: how fast data moves between this computer and the device's SD card, each leg
apart, so that a slow copy says where its time goes: a file put onto the card; the
console alone, the glass sent from the device's memory; the card read through the
console, as a backup does; a copy on the card, as Habity's sounds are copied. Read and
copied: the largest of Habity's ringtones, else the file put."""
import argparse
import time
from dataclasses import dataclass

from . import card, observation
from .command import on_link
from .console import ConsoleError

# Under the device's own directory, removed once measured.
BENCH_DIRECTORY = 'cute-display/bench'
PUT = f'{BENCH_DIRECTORY}/put'
COPIED = f'{BENCH_DIRECTORY}/copy'
# The most a put takes, src/maintenance/'s MAX_PUT_BYTES.
PUT_BYTES = 64 * 1024
# A few megabytes each: read and copied whole, with little of a request's own time.
RINGTONES = 'sounds/alarm'
# Read and copied as many times as it takes, the file a megabyte or less of it.
MOVED_BYTES = 1024 * 1024
SCREENS = 8


@dataclass(frozen=True)
class Leg:
    name: str
    size: int
    seconds: float

    def __str__(self):
        speed = self.size / 1024 / max(self.seconds, 1e-6)
        return f'{self.name:<40} {speed:>7.0f} KB/s   {self.size / 1024:>6.0f} KB in {self.seconds:.1f} s'


def timed(name, move, now):
    """`move()` gives how many bytes it moved."""
    started = now()
    size = move()
    return Leg(name, size, now() - started)


def card_put(link):
    card.write_file(link, PUT, bytes(range(256)) * (PUT_BYTES // 256))
    return PUT_BYTES


def source(link):
    """The largest ringtone and its size, else the file put."""
    try:
        files = [e for e in card.entries(link, RINGTONES) if not e.is_directory]
    except ConsoleError:
        files = []
    if not files:
        return PUT, PUT_BYTES
    largest = max(files, key=lambda e: e.size)
    return f'{RINGTONES}/{largest.name}', largest.size


def console_alone(link):
    """The glass refuses while it is drawn: only the frames that came whole count."""
    size = 0
    for _ in range(SCREENS):
        try:
            size += len(observation.screen(link).frame.ink)
        except ConsoleError:
            pass
    return size


def card_read(link, path):
    size = offset = 0
    while size < MOVED_BYTES:
        contents = card.read_range(link, path, offset)
        offset = offset + len(contents) if contents else 0
        size += len(contents)
    return size


def card_copy(link, path, file_size):
    size = 0
    while size < MOVED_BYTES:
        card.copy(link, path, COPIED)
        size += file_size
    return size


def legs(link, now=time.monotonic):
    """Each leg as it is measured."""
    try:
        yield timed('put onto the card, through the console', lambda: card_put(link), now)
        yield timed('console alone (the glass, from memory)', lambda: console_alone(link), now)
        path, size = source(link)
        yield timed('card read, through the console', lambda: card_read(link, path), now)
        yield timed('copy on the card', lambda: card_copy(link, path, size), now)
    finally:
        for written in (PUT, COPIED):
            try:
                card.remove(link, written)
            except ConsoleError:
                pass


def measure(link):
    print(f'read and copied: {source(link)[0]}')
    for leg in legs(link):
        print(leg, flush=True)


def main():
    parser = argparse.ArgumentParser(prog='bench', description=__doc__)
    parser.add_argument('--simulator', metavar='SOCKET', help="a simulator's, started with --console SOCKET")
    arguments = parser.parse_args()
    on_link(arguments.simulator, measure)
