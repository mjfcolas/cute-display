"""The time that the test image asks for over NTP (infrastructure::testing::to_local_stubs):
the time that a scenario sets, running from there."""
import socket
import struct
import threading
import time

# Above 1023, so that a normal user can listen on it.
NTP_PORT = 8123
NTP_PACKET_BYTES = 48
# From 1900-01-01, where NTP counts from, to 1970-01-01.
NTP_TO_UNIX_SECONDS = 2_208_988_800
# Leap indicator 0, version 4, mode 4: a server; stratum 2.
NTP_SERVER_HEADER = bytes([0b00_100_100, 2])
NTP_TRANSMIT_TIMESTAMP = 40


def ntp_answer(unix_seconds):
    packet = bytearray(NTP_PACKET_BYTES)
    packet[:2] = NTP_SERVER_HEADER
    struct.pack_into('>I', packet, NTP_TRANSMIT_TIMESTAMP, (int(unix_seconds) + NTP_TO_UNIX_SECONDS) % (1 << 32))
    return bytes(packet)


class FakeRunningTime:
    def __init__(self, unix_seconds=None):
        self._lock = threading.Lock()
        self.set(time.time() if unix_seconds is None else unix_seconds)

    def set(self, unix_seconds):
        with self._lock:
            self._at, self._since = unix_seconds, time.monotonic()

    def now(self):
        with self._lock:
            return self._at + time.monotonic() - self._since


class StubTimeServer:
    """Port 0 lets the system choose. Read the chosen port from `port`."""

    def __init__(self, *, port=NTP_PORT, running_time=None):
        self.time = running_time or FakeRunningTime()
        self._socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self._socket.bind(('', port))
        # On Linux, closing a socket that another thread waits on leaves it bound until a
        # datagram arrives. So the thread waits a short time at a time, and sees that it
        # should stop.
        self._socket.settimeout(0.1)
        self.port = self._socket.getsockname()[1]
        self._stopping = threading.Event()
        self._thread = threading.Thread(target=self._serve, daemon=True)
        self._thread.start()

    def close(self):
        self._stopping.set()
        self._thread.join()
        self._socket.close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()

    def _serve(self):
        while not self._stopping.is_set():
            try:
                _, sender = self._socket.recvfrom(NTP_PACKET_BYTES * 2)
            except TimeoutError:
                continue
            self._socket.sendto(ntp_answer(self.time.now()), sender)
