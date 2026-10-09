"""Requests to the device's console (src/maintenance/, whose lib.rs describes the lines), on
any link that writes, flushes and reads lines as pyserial does: `readline` returns `b''`
when nothing came in time."""
import random
import time

REPLY_TIMEOUT_S = 10


class ConsoleError(Exception):
    """The device refused a request, or did not answer it completely."""


class Request:
    """One request and its replies, told apart from the others by a random id."""

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
            # The device's console does not wait for a log line to be fully written: a reply
            # may appear in the middle of a log line, and the id identifies it.
            at = line.find(prefix)
            if at >= 0:
                deadline = time.monotonic() + self.timeout_s
                kind, _, rest = line[at + len(prefix):].partition(' ')
                if kind == 'error':
                    raise ConsoleError(rest)
                yield kind, rest
        raise ConsoleError('no answer from the device; is Cute Display running, and no serial monitor open?')

    def expect(self, wanted):
        kind, rest = next(self.replies())
        if kind != wanted:
            raise ConsoleError(f'expected {wanted}, got {kind} {rest}')
        return rest


class KeepingLog:
    """A link that keeps the lines that are not the console's (the device's log), for a
    diagnostic and to detect a crash or a restart. They arrive while requests are answered.
    """

    def __init__(self, link):
        self.link = link
        self.kept = []

    def write(self, data):
        self.link.write(data)

    def flush(self):
        self.link.flush()

    def readline(self):
        line = self.link.readline()
        text = line.decode(errors='replace').rstrip('\r\n')
        if text and not text.startswith('@@ '):
            self.kept.append(text)
        return line

    def take_log(self):
        taken, self.kept = self.kept, []
        return taken
