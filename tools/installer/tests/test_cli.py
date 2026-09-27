import unittest

from cute_display_installer.cli import explain


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
