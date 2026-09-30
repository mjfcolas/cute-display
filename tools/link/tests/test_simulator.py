import socket
import tempfile
import unittest
from pathlib import Path

from cute_display_link.simulator import SimulatorLink


class SimulatorLinkTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        path = Path(self.directory.name) / 'console.sock'
        self.server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.server.bind(str(path))
        self.server.listen()
        self.link = SimulatorLink(path)
        self.simulator, _ = self.server.accept()

    def tearDown(self):
        self.link.close()
        self.simulator.close()
        self.server.close()
        self.directory.cleanup()

    def test_lines_come_whole_however_they_arrive(self):
        self.simulator.sendall(b'@@ 1 o')
        self.assertEqual(self.link.readline(), b'')
        self.simulator.sendall(b'k\n@@ 2 ok\n')
        self.assertEqual(self.link.readline(), b'@@ 1 ok\n')
        self.assertEqual(self.link.readline(), b'@@ 2 ok\n')

    def test_what_is_written_reaches_the_simulator(self):
        self.link.write(b'@@ 1 tap yellow\n')
        self.assertEqual(self.simulator.recv(100), b'@@ 1 tap yellow\n')
