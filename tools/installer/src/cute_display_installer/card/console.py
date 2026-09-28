"""The device's SD card, through the maintenance console of the running app image.

The other end is src/maintenance/, whose lib.rs describes the lines. Everything can be
read; only `cute-display/` can be written or removed.
"""
import base64
import random
import time
import zlib
from dataclasses import dataclass

# A data line waits for its ack, so it only has to fit in the device's 1024-byte receive
# buffer: 240 bytes are 320 characters of base64.
CHUNK_BYTES = 240
REPLY_TIMEOUT_S = 10
# A copy is made on the device, which answers once it is whole: seconds a megabyte.
COPY_TIMEOUT_S = 120
# The console drops output nobody reads in time: a range that arrives damaged is asked again.
RANGE_ATTEMPTS = 5


class ConsoleError(Exception):
    """The device refused a request, or did not answer it whole."""


@dataclass(frozen=True)
class Entry:
    name: str
    is_directory: bool
    size: int


class _Request:
    """One request and its replies, told apart from the others' by a random id."""

    def __init__(self, link, words, timeout_s=REPLY_TIMEOUT_S):
        self.link = link
        self.id = str(random.randrange(1, 1_000_000))
        self.timeout_s = timeout_s
        self.send(words)

    def send(self, words):
        self.link.write(f'@@ {self.id} {words}\n'.encode())
        self.link.flush()

    def replies(self):
        prefix = f'@@ {self.id} '
        deadline = time.monotonic() + self.timeout_s
        while time.monotonic() < deadline:
            line = self.link.readline().decode(errors='replace').strip()
            if line.startswith(prefix):
                deadline = time.monotonic() + self.timeout_s
                kind, _, rest = line[len(prefix):].partition(' ')
                if kind == 'error':
                    raise ConsoleError(rest)
                yield kind, rest
        raise ConsoleError('no answer from the device; is Cute Display running, and no serial monitor open?')

    def expect(self, wanted):
        kind, rest = next(self.replies())
        if kind != wanted:
            raise ConsoleError(f'expected {wanted}, got {kind} {rest}')
        return rest


def entries(link, directory=''):
    found = []
    for kind, rest in _Request(link, f'ls {directory}').replies():
        if kind == 'ok':
            return found
        entry_type, size, name = rest.split(' ', 2)
        found.append(Entry(name=name, is_directory=entry_type == 'd', size=int(size)))


def _read_range(link, path, offset):
    for _ in range(RANGE_ATTEMPTS):
        contents = bytearray()
        for kind, rest in _Request(link, f'get {offset} {path}').replies():
            if kind == 'data':
                contents += base64.b64decode(rest)
            elif kind == 'ok':
                size, crc = (int(n) for n in rest.split())
                if len(contents) == size and zlib.crc32(contents) == crc:
                    return bytes(contents)
                break
    raise ConsoleError(f'{path} keeps arriving damaged at byte {offset}')


def read_file(link, path):
    contents = bytearray()
    while chunk := _read_range(link, path, len(contents)):
        contents += chunk
    return bytes(contents)


def write_file(link, path, contents):
    request = _Request(link, f'put {len(contents)} {zlib.crc32(contents)} {path}')
    request.expect('ready')
    for at in range(0, len(contents), CHUNK_BYTES):
        request.send(f'data {base64.b64encode(contents[at:at + CHUNK_BYTES]).decode()}')
        request.expect('ack')
    request.send('end')
    request.expect('ok')


def copy(link, source, destination):
    """On the device, from anywhere on the card into cute-display/."""
    _Request(link, f'cp {source}\t{destination}', COPY_TIMEOUT_S).expect('ok')


def remove(link, path):
    _Request(link, f'rm {path}').expect('ok')
