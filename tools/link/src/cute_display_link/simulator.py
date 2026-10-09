"""The simulator's console, on the Unix socket that it listens on (`--console`)."""
import socket

from .lines import Lines

READ_TIMEOUT_S = 0.2


class NoSimulator(Exception):
    pass


class SimulatorLink:
    def __init__(self, path):
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            self.socket.connect(str(path))
        except (FileNotFoundError, ConnectionRefusedError):
            self.socket.close()
            raise NoSimulator(f'No simulator on {path}: start one with --console {path}.') from None
        self.socket.settimeout(READ_TIMEOUT_S)
        self.lines = Lines(self._receive)

    def write(self, data):
        self.socket.sendall(data)

    def flush(self):
        pass

    def _receive(self):
        try:
            return self.socket.recv(4096)
        except TimeoutError:
            return b''

    def readline(self):
        return self.lines.readline()

    def close(self):
        self.socket.close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()
