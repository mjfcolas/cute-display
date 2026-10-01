import unittest
from unittest import mock

from cute_display_e2e import waiting
from cute_display_e2e.waiting import until


class Until(unittest.TestCase):
    def test_gives_what_the_condition_gave_once_it_gave_something(self):
        answers = iter([None, [], 'found'])
        with mock.patch.object(waiting.time, 'sleep'):
            self.assertEqual(until(lambda: next(answers), 10, 'never'), 'found')

    def test_fails_saying_what_did_not_happen(self):
        with self.assertRaisesRegex(AssertionError, 'the alarm did not ring, in 0 s'):
            until(lambda: False, 0, 'the alarm did not ring')
