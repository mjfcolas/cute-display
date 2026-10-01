import unittest

from cute_display_e2e.screen import Screen

SYSTEM = Screen(('front system', 'title System', 'app Alarm clock', 'app Weather *', 'setting Reading lamp off'))


class Lines(unittest.TestCase):
    def test_the_app_in_front_is_the_first_line(self):
        self.assertEqual(SYSTEM.front(), 'system')
        self.assertIsNone(Screen(()).front(), 'before the first frame')

    def test_a_line_is_found_by_its_name_without_its_mark(self):
        self.assertEqual(SYSTEM.first_value('app'), 'Alarm clock')
        self.assertEqual(SYSTEM.values('app'), ['Alarm clock', 'Weather'])
        self.assertIsNone(SYSTEM.first_value('hint'))

    def test_the_line_chosen_is_the_one_marked(self):
        self.assertEqual(SYSTEM.chosen(), 'app Weather')
        self.assertIsNone(Screen(('front alarm',)).chosen())
