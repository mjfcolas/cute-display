"""The device's SD card, through its console."""
import base64
import zlib
from dataclasses import dataclass

from .console import ConsoleError, Request

# A data line waits for its ack, so it only has to fit in the device's 1024-byte receive
# buffer: 240 bytes are 320 characters of base64.
CHUNK_BYTES = 240
# A copy is made on the device, which answers once it is whole: seconds a megabyte.
COPY_TIMEOUT_S = 120
# The console drops output nobody reads in time: a range that arrives damaged is asked again.
RANGE_ATTEMPTS = 5


@dataclass(frozen=True)
class Entry:
    name: str
    is_directory: bool
    size: int


def entries(link, directory=''):
    found = []
    for kind, rest in Request(link, f'ls {directory}').replies():
        if kind == 'ok':
            return found
        entry_type, size, name = rest.split(' ', 2)
        found.append(Entry(name=name, is_directory=entry_type == 'd', size=int(size)))


def _read_range(link, path, offset):
    for _ in range(RANGE_ATTEMPTS):
        contents = bytearray()
        for kind, rest in Request(link, f'get {offset} {path}').replies():
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
    request = Request(link, f'put {len(contents)} {zlib.crc32(contents)} {path}')
    request.expect('ready')
    for at in range(0, len(contents), CHUNK_BYTES):
        request.send(f'data {base64.b64encode(contents[at:at + CHUNK_BYTES]).decode()}')
        request.expect('ack')
    request.send('end')
    request.expect('ok')


def copy(link, source, destination):
    """On the device, from anywhere on the card into cute-display/."""
    Request(link, f'cp {source}\t{destination}', COPY_TIMEOUT_S).expect('ok')


def remove(link, path):
    Request(link, f'rm {path}').expect('ok')
