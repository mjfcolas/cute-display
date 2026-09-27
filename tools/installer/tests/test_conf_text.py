import unittest

from cute_display_installer.config.conf_text import parse


class ConfText(unittest.TestCase):
    def test_keys_and_values_are_trimmed_and_the_rest_ignored(self):
        conf = parse('# where\nplace =  Paris \nlatitude=48.85\nnonsense\n')
        self.assertEqual(conf, {'place': 'Paris', 'latitude': '48.85'})

    def test_a_value_may_hold_an_equals_sign_and_the_last_one_wins(self):
        self.assertEqual(parse('password = a=b=c\npassword = d')['password'], 'd')
        self.assertEqual(parse('password = a=b=c')['password'], 'a=b=c')


if __name__ == '__main__':
    unittest.main()
