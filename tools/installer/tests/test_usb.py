import os
import unittest
from types import SimpleNamespace
from unittest import mock

from cute_display_installer.usb import PORT_VARIABLE, NoDevice, port

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


if __name__ == '__main__':
    unittest.main()
