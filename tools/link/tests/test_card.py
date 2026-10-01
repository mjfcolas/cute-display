import unittest

from cute_display_link.card import COPY_TIMEOUT_S, MEGABYTE, Entry, copy, copy_timeout_s, entries, read_file, remove, write_file
from cute_display_link.console import ConsoleError
from cute_display_link_testing.fake_card import FakeCard
from cute_display_link_testing.fake_console import FakeConsole


class CardFiles(unittest.TestCase):
    def test_a_directory_lists_its_entries(self):
        device = FakeConsole(FakeCard({'cute-display/wifi.conf': b'ssid = Home\n', 'sounds/alarm.wav': b''}))
        self.assertEqual(entries(device), [Entry('cute-display', True, 0), Entry('sounds', True, 0)])
        self.assertEqual(entries(device, 'cute-display'), [Entry('wifi.conf', False, 12)])

    def test_a_file_longer_than_a_range_is_read_whole(self):
        contents = bytes(range(256)) * 3
        self.assertEqual(read_file(FakeConsole(FakeCard({'sounds/alarm.wav': contents})), 'sounds/alarm.wav'), contents)

    def test_a_damaged_range_is_asked_again(self):
        contents = b'x' * 250
        self.assertEqual(read_file(FakeConsole(FakeCard({'a': contents}, damaged_ranges=2)), 'a'), contents)

    def test_a_file_longer_than_a_data_line_is_written_whole(self):
        card = FakeCard({})
        contents = bytes(range(256)) * 4
        write_file(FakeConsole(card), 'cute-display/airports.conf', contents)
        self.assertEqual(card.files['cute-display/airports.conf'], contents)

    def test_what_the_device_refuses_is_said(self):
        with self.assertRaisesRegex(ConsoleError, 'only cute-display'):
            write_file(FakeConsole(FakeCard({})), 'sounds/alarm.wav', b'')

    def test_a_file_is_copied_on_the_device(self):
        card = FakeCard({'sounds/alarm/Lost Ark.mp3': b'ID3'})
        device = FakeConsole(card)
        copy(device, 'sounds/alarm/Lost Ark.mp3', 'cute-display/apps/alarm/ringtones/Lost Ark.mp3')
        self.assertEqual(card.files['cute-display/apps/alarm/ringtones/Lost Ark.mp3'], b'ID3')
        with self.assertRaisesRegex(ConsoleError, 'only cute-display'):
            copy(device, 'sounds/alarm/Lost Ark.mp3', 'sounds/Lost Ark.mp3')

    def test_a_large_copy_is_waited_for_longer(self):
        self.assertEqual(copy_timeout_s(3 * MEGABYTE), COPY_TIMEOUT_S)
        self.assertGreater(copy_timeout_s(30 * MEGABYTE), COPY_TIMEOUT_S)

    def test_a_file_is_removed(self):
        card = FakeCard({'cute-display/old.conf': b''})
        remove(FakeConsole(card), 'cute-display/old.conf')
        self.assertEqual(card.files, {})


if __name__ == '__main__':
    unittest.main()
