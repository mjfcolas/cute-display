import unittest
from unittest import mock

from cute_display_link import controls
from cute_display_link.console import REPLY_TIMEOUT_S, ConsoleError
from cute_display_link_testing.fake_card import Card


class Controls(unittest.TestCase):
    def test_each_control_is_one_request(self):
        card = Card({})
        controls.tap(card, 'yellow')
        controls.hold_until_released(card, 1500, ['yellow', 'long'])
        controls.turn(card, -2)
        self.assertEqual(card.controls, ['tap yellow', 'hold 1500 yellow long', 'turn -2'])


class Late:
    """A device that answers `answer_after_s` late, on a clock that moves a second each
    time nothing came."""

    def __init__(self, answer_after_s):
        self.card = Card({})
        self.now = 0.0
        self.answer_after_s = answer_after_s

    def monotonic(self):
        return self.now

    def write(self, data):
        self.card.write(data)

    def flush(self):
        pass

    def readline(self):
        if self.now < self.answer_after_s:
            self.now += 1
            return b''
        return self.card.readline()


class Waiting(unittest.TestCase):
    def test_a_hold_waits_for_its_release_beyond_the_usual_wait(self):
        device = Late(answer_after_s=REPLY_TIMEOUT_S + 5)
        with mock.patch('cute_display_link.console.time.monotonic', device.monotonic):
            controls.hold_until_released(device, 8000, ['long'])
        self.assertEqual(device.card.controls, ['hold 8000 long'])

    def test_a_tap_answered_that_late_is_unanswered(self):
        device = Late(answer_after_s=REPLY_TIMEOUT_S + 5)
        with mock.patch('cute_display_link.console.time.monotonic', device.monotonic):
            with self.assertRaises(ConsoleError):
                controls.tap(device, 'yellow')
