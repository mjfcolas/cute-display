import itertools
import unittest
from contextlib import nullcontext
from unittest import mock

from cute_display_link import bench, usb
from cute_display_link.bench import MOVED_BYTES, PUT_BYTES, SCREENS, legs
from cute_display_link.frame import BYTES
from cute_display_link_testing.fake_card import RANGE_BYTES, FakeCard
from cute_display_link_testing.fake_console import FakeConsole
from cute_display_link_testing.fake_observation import FakeObservation


def a_second_a_call():
    return itertools.count().__next__


def device(card=None):
    return FakeConsole(card if card is not None else FakeCard({}), FakeObservation(ink=bytes(BYTES)))


class Legs(unittest.TestCase):
    def test_each_leg_moves_its_bytes_in_the_time_it_took(self):
        measured = list(legs(device(), now=a_second_a_call()))
        self.assertEqual([(leg.size, leg.seconds) for leg in measured],
                         [(PUT_BYTES, 1), (SCREENS * BYTES, 1), (MOVED_BYTES, 1), (MOVED_BYTES, 1)])

    def test_the_largest_ringtone_is_read_and_copied_whole(self):
        ringtones = {'sounds/alarm/Default.mp3': b'ID3', 'sounds/alarm/Zen.mp3': bytes(MOVED_BYTES + 3)}
        measured = list(legs(device(FakeCard(ringtones)), now=a_second_a_call()))
        read, copied = (leg.size for leg in measured[2:])
        self.assertTrue(MOVED_BYTES <= read < MOVED_BYTES + RANGE_BYTES, read)
        self.assertEqual(copied, MOVED_BYTES + 3)
        self.assertEqual(bench.source(device(FakeCard(ringtones))), ('sounds/alarm/Zen.mp3', MOVED_BYTES + 3))

    def test_what_it_wrote_is_removed_from_the_card(self):
        card = FakeCard({'sounds/alarm/Zen.mp3': b'ID3'})
        list(legs(device(card), now=a_second_a_call()))
        self.assertEqual(list(card.files), ['sounds/alarm/Zen.mp3'])

    def test_what_it_wrote_is_removed_even_when_it_stops_early(self):
        card = FakeCard({})
        measuring = legs(device(card), now=a_second_a_call())
        next(measuring)
        measuring.close()
        self.assertEqual(card.files, {})

    def test_a_glass_that_refuses_counts_for_nothing(self):
        self.assertEqual(bench.console_alone(FakeConsole(FakeObservation())), 0)


class Command(unittest.TestCase):
    def test_each_leg_is_printed_by_name(self):
        printed = []
        with mock.patch.object(usb, 'open_link', return_value=nullcontext(device())), \
                mock.patch('builtins.print', lambda line, **_: printed.append(str(line))), mock.patch('sys.argv', ['bench']):
            bench.main()
        self.assertEqual([line.split('  ')[0] for line in printed],
                         ['read and copied: cute-display/bench/put', 'put onto the card, through the console', 'console alone (the glass, from memory)',
                          'card read, through the console', 'copy on the card'])
