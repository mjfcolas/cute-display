"""Lines cut from what came in, whatever it came in: pyserial's own readline reads a byte
at a time, which caps the cable at about 200 KB/s."""


class Lines:
    """`readline` gives a whole line, or `b''` when none came in time, and keeps what came
    of one for the next; `receive()` gives what came in, `b''` when nothing did in time."""

    def __init__(self, receive):
        self.receive = receive
        self.pending = bytearray()

    def readline(self):
        while (end := self.pending.find(b'\n')) < 0:
            received = self.receive()
            if not received:
                return b''
            self.pending += received
        line = bytes(self.pending[:end + 1])
        del self.pending[:end + 1]
        return line

    def clear(self):
        self.pending.clear()
