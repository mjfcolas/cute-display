"""Lines cut from what came in, however it came in: pyserial's own readline reads one byte
at a time, which limits the cable to about 200 KB/s."""


class Lines:
    """`readline` returns a whole line, or `b''` when no line came in time, and keeps a
    partial line for the next call. `receive()` returns what came in, or `b''` when nothing
    came in time."""

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
