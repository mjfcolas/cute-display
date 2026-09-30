import unittest

from cute_display_link import observation
from cute_display_link_testing.fake_console import FakeConsole
from cute_display_link_testing.fake_observation import FakeObservation


class Observation(unittest.TestCase):
    def test_the_lights_and_the_speaker_are_read(self):
        device = FakeConsole(FakeObservation(lights=(20, 50), playing=True))
        self.assertEqual(observation.lights(device), (20, 50))
        self.assertTrue(observation.is_playing(device))
