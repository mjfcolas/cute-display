import unittest

from cute_display_link import observation
from cute_display_link.console import ConsoleError
from cute_display_link.frame import BYTES
from cute_display_link_testing.fake_console import FakeConsole
from cute_display_link_testing.fake_observation import FakeObservation


class Observation(unittest.TestCase):
    def test_the_lights_and_the_speaker_are_read(self):
        device = FakeConsole(FakeObservation(lights=(20, 50), playing=True))
        self.assertEqual(observation.lights(device), (20, 50))
        self.assertTrue(observation.is_playing(device))

    def test_the_screen_comes_whole_with_the_times_it_was_shown(self):
        ink = bytes(range(256)) * (BYTES // 256) + bytes(BYTES % 256)
        screen = observation.screen(FakeConsole(FakeObservation(ink=ink, times_shown=7)))
        self.assertEqual((screen.times_shown, screen.frame.ink), (7, ink))

    def test_a_screen_never_shown_is_said(self):
        with self.assertRaisesRegex(ConsoleError, 'nothing shown'):
            observation.screen(FakeConsole(FakeObservation()))
