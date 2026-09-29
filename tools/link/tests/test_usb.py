import os
import unittest
from types import SimpleNamespace
from unittest import mock

from cute_display_link.usb import PORT_VARIABLE, NoDevice, explain, port

HABITY = SimpleNamespace(device='/dev/ttyACM0', vid=0x303a, pid=0x1001)
OTHER = SimpleNamespace(device='/dev/ttyUSB0', vid=0x10c4, pid=0xea60)


@mock.patch.dict(os.environ, {}, clear=True)
class Port(unittest.TestCase):
    def test_the_one_espressif_port_is_the_device(self):
        self.assertEqual(port([OTHER, HABITY]), '/dev/ttyACM0')

    def test_without_it_the_cable_is_suspected(self):
        with self.assertRaisesRegex(NoDevice, 'carry data'):
            port([OTHER])

    def test_two_devices_need_choosing(self):
        with self.assertRaisesRegex(NoDevice, PORT_VARIABLE):
            port([HABITY, SimpleNamespace(device='/dev/ttyACM1', vid=0x303a, pid=0x1001)])

    def test_the_variable_wins(self):
        with mock.patch.dict(os.environ, {PORT_VARIABLE: 'COM7'}):
            self.assertEqual(port([HABITY]), 'COM7')


class Explain(unittest.TestCase):
    def test_a_busy_port_asks_for_the_monitor_to_be_closed(self):
        for message in ("could not open port /dev/ttyACM0: [Errno 16] Device or resource busy",
                        "could not open port 'COM7': PermissionError(13, 'Access is denied.', None, 5)"):
            self.assertIn('close any serial monitor', explain(Exception(message)))

    def test_a_forbidden_port_asks_for_the_group(self):
        self.assertIn('dialout or uucp', explain(Exception('[Errno 13] Permission denied: /dev/ttyACM0')))

    def test_anything_else_asks_for_the_cable_again(self):
        self.assertIn('Unplug it', explain(Exception('Failed to connect to ESP32-S3: No serial data received.')))


if __name__ == '__main__':
    unittest.main()
