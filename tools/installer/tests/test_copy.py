import os
import tempfile
import unittest

from cute_display_installer.card.copy import pull
from cute_display_link_testing.fake_card import Card


class Pull(unittest.TestCase):
    def test_the_tree_is_copied_and_what_is_there_at_the_same_size_skipped(self):
        card = Card({'cute-display/wifi.conf': b'ssid = Home\n', 'sounds/rain/1.wav': b'drops'})
        reported = []
        with tempfile.TemporaryDirectory() as local:
            os.makedirs(os.path.join(local, 'cute-display'))
            with open(os.path.join(local, 'cute-display', 'wifi.conf'), 'wb') as f:
                f.write(b'ssid = Away\n')
            pull(card, '', local, reported.append)
            with open(os.path.join(local, 'sounds', 'rain', '1.wav'), 'rb') as f:
                self.assertEqual(f.read(), b'drops')
            with open(os.path.join(local, 'cute-display', 'wifi.conf'), 'rb') as f:
                self.assertEqual(f.read(), b'ssid = Away\n')
            left = [name for _, _, names in os.walk(local) for name in names if name.endswith('.part')]
            self.assertEqual(left, [])
        self.assertIn('cute-display/wifi.conf: already there', reported)


if __name__ == '__main__':
    unittest.main()
