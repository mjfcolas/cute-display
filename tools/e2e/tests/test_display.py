import unittest
from unittest import mock

from cute_display_e2e import waiting
from cute_display_e2e.display import Display, Lights
from cute_display_e2e.screen import Screen
from cute_display_link_testing.fake_console import FakeConsole
from cute_display_link_testing.fake_observation import FakeObservation


def display(**observed):
    return Display(FakeConsole(FakeObservation(**observed)))


class WhatTheGlassSays(unittest.TestCase):
    def test_before_the_first_frame_the_glass_says_nothing(self):
        self.assertEqual(display().screen(), Screen(()))

    def test_the_screen_waited_for_is_given_once_it_comes(self):
        self.assertEqual(display(lines=['front weather']).until_front('weather'), Screen(('front weather',)))

    def test_a_screen_that_does_not_come_fails_quoting_the_last_one_seen(self):
        with mock.patch.object(waiting.time, 'sleep'), \
                self.assertRaisesRegex(AssertionError, 'weather did not come to the front, in 0 s; the glass said:\nfront alarm'):
            display(lines=['front alarm']).until_screen(lambda screen: screen.front() == 'weather', 'weather did not come to the front', timeout_s=0)


class TheLights(unittest.TestCase):
    def test_are_said_by_name(self):
        self.assertEqual(display(lights=(20, 10)).lights(), Lights(front_percent=20, reading_lamp_percent=10))
