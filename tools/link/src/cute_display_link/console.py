"""Requests to the device's console, src/maintenance/, whose lib.rs describes the lines,
on any link that writes, flushes and reads lines as pyserial does: `readline` gives
`b''` when nothing came in time."""
import random
import time

REPLY_TIMEOUT_S = 10


class ConsoleError(Exception):
    """The device refused a request, or did not answer it whole."""


class Request:
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
