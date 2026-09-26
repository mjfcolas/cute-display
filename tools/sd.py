#!/usr/bin/env python3
"""Read the device's SD card, and put files in its cute-display/ directory, over the USB cable.

  tools/sd.py ls [dir]
  tools/sd.py get <path on the card> [local file]     (to stdout without a local file)
  tools/sd.py put <local file> <path on the card>
  tools/sd.py rm <path on the card>

The device must run the app image, and nothing else may hold the port (close the
monitor first). The port is found by its USB name; CUTE_DISPLAY_PORT overrides it.
The other end is src/maintenance/src/console.rs.
"""
import base64
import glob
import os
import random
import sys
import time
import zlib

import serial

PORT_PATTERN = '/dev/serial/by-id/*Espressif*'
CHUNK_BYTES = 48
REPLY_TIMEOUT_S = 10


class Refused(Exception):
    pass


def open_port():
    port = os.environ.get('CUTE_DISPLAY_PORT') or next(iter(sorted(glob.glob(PORT_PATTERN))), None)
    if not port:
        sys.exit(f'no device found at {PORT_PATTERN}; is it plugged in?')
    # Opening the port raises DTR and RTS, and pyserial's defaults keep them there. On
    # the ESP32-S3, lowering one before the other is what resets the chip.
    link = serial.Serial(port, 115200, timeout=0.2)
    link.reset_input_buffer()
    return link


class Console:
    def __init__(self, link):
        self.link = link
        self.id = str(random.randrange(1, 1_000_000))

    def send(self, words):
        self.link.write(f'@@ {self.id} {words}\n'.encode())
        self.link.flush()

    def replies(self):
        """This request's replies, one at a time, skipping the device's log."""
        prefix = f'@@ {self.id} '
        deadline = time.monotonic() + REPLY_TIMEOUT_S
        while time.monotonic() < deadline:
            line = self.link.readline().decode(errors='replace').strip()
            if line.startswith(prefix):
                deadline = time.monotonic() + REPLY_TIMEOUT_S
                kind, _, rest = line[len(prefix):].partition(' ')
                if kind == 'error':
                    raise Refused(rest)
                yield kind, rest
        raise Refused('no answer from the device; is the app image running?')

    def expect(self, wanted):
        kind, rest = next(self.replies())
        if kind != wanted:
            raise Refused(f'expected {wanted}, got {kind} {rest}')
        return rest


def ls(link, directory=''):
    console = Console(link)
    console.send(f'ls {directory}')
    for kind, rest in console.replies():
        if kind == 'ok':
            return
        entry_type, size, name = rest.split(' ', 2)
        print(f'{"d" if entry_type == "d" else "-"} {int(size):>10} {name}')


def get(link, path, destination=None):
    console = Console(link)
    console.send(f'get {path}')
    contents = bytearray()
    for kind, rest in console.replies():
        if kind == 'data':
            contents += base64.b64decode(rest)
        elif kind == 'ok':
            size, crc = (int(n) for n in rest.split())
            if len(contents) != size or zlib.crc32(contents) != crc:
                raise Refused('the file arrived damaged; try again')
            break
    if destination:
        with open(destination, 'wb') as f:
            f.write(contents)
        print(f'{path} -> {destination} ({len(contents)} bytes)')
    else:
        sys.stdout.buffer.write(contents)


def put(link, source, path):
    with open(source, 'rb') as f:
        contents = f.read()
    console = Console(link)
    console.send(f'put {path} {len(contents)} {zlib.crc32(contents)}')
    console.expect('ready')
    for at in range(0, len(contents), CHUNK_BYTES):
        console.send(f'data {base64.b64encode(contents[at:at + CHUNK_BYTES]).decode()}')
        console.expect('ack')
    console.send('end')
    console.expect('ok')
    print(f'{source} -> {path} ({len(contents)} bytes)')


def rm(link, path):
    console = Console(link)
    console.send(f'rm {path}')
    console.expect('ok')
    print(f'removed {path}')


COMMANDS = {'ls': (ls, 0, 1), 'get': (get, 1, 2), 'put': (put, 2, 2), 'rm': (rm, 1, 1)}


def main():
    if len(sys.argv) < 2 or sys.argv[1] not in COMMANDS:
        sys.exit(__doc__)
    command, least, most = COMMANDS[sys.argv[1]]
    args = sys.argv[2:]
    if not least <= len(args) <= most:
        sys.exit(__doc__)
    with open_port() as link:
        try:
            command(link, *args)
        except Refused as refused:
            sys.exit(f'{sys.argv[1]}: {refused}')


if __name__ == '__main__':
    main()
