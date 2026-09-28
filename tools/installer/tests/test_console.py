import unittest

from cute_display_installer.card.console import ConsoleError, Entry, copy, entries, read_file, remove, write_file
from fake_card import Card


class Console(unittest.TestCase):
    def test_a_directory_lists_its_entries(self):
        card = Card({'cute-display/wifi.conf': b'ssid = Home\n', 'sounds/alarm.wav': b''})
        self.assertEqual(entries(card), [Entry('cute-display', True, 0), Entry('sounds', True, 0)])
        self.assertEqual(entries(card, 'cute-display'), [Entry('wifi.conf', False, 12)])

    def test_a_file_longer_than_a_range_is_read_whole(self):
        contents = bytes(range(256)) * 3
        self.assertEqual(read_file(Card({'sounds/alarm.wav': contents}), 'sounds/alarm.wav'), contents)

    def test_a_damaged_range_is_asked_again(self):
        contents = b'x' * 250
        self.assertEqual(read_file(Card({'a': contents}, damaged_ranges=2), 'a'), contents)

    def test_a_file_longer_than_a_data_line_is_written_whole(self):
        card = Card({})
        contents = bytes(range(256)) * 4
        write_file(card, 'cute-display/airports.conf', contents)
        self.assertEqual(card.files['cute-display/airports.conf'], contents)

    def test_what_the_device_refuses_is_said(self):
        with self.assertRaisesRegex(ConsoleError, 'only cute-display'):
            write_file(Card({}), 'sounds/alarm.wav', b'')

    def test_a_file_is_copied_on_the_device(self):
        card = Card({'sounds/alarm/Lost Ark.mp3': b'ID3'})
        copy(card, 'sounds/alarm/Lost Ark.mp3', 'cute-display/apps/alarm/ringtones/Lost Ark.mp3')
        self.assertEqual(card.files['cute-display/apps/alarm/ringtones/Lost Ark.mp3'], b'ID3')
        with self.assertRaisesRegex(ConsoleError, 'only cute-display'):
            copy(card, 'sounds/alarm/Lost Ark.mp3', 'sounds/Lost Ark.mp3')

    def test_a_file_is_removed(self):
        card = Card({'cute-display/old.conf': b''})
        remove(card, 'cute-display/old.conf')
        self.assertEqual(card.files, {})


if __name__ == '__main__':
    unittest.main()
