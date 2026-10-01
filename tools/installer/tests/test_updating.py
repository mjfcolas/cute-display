import unittest

from cute_display_installer.flash.layout import AppImage
from cute_display_installer.updating import Entry, entries, to_know

TEXT = '''# Updating

What to know before updating.

## 2026.10.0

- The radar's airports are fetched anew.

## Notes

- Not a version.

## 2026.9.1

- The alarm's settings are lost: set them again.
'''
ALARM = Entry('2026.9.1', "- The alarm's settings are lost: set them again.")
RADAR = Entry('2026.10.0', "- The radar's airports are fetched anew.")


def image(version):
    return AppImage('cute-display', version)


class Entries(unittest.TestCase):
    def test_each_version_is_an_entry_and_nothing_else_is(self):
        self.assertEqual(entries(TEXT), (RADAR, ALARM))

    def test_a_file_without_entries_has_none(self):
        self.assertEqual(entries('# Updating\n\nWhat to know.\n'), ())


class ToKnow(unittest.TestCase):
    def test_the_entries_after_the_installed_version_up_to_the_latest_oldest_first(self):
        self.assertEqual(to_know(entries(TEXT), image('2026.9.0'), image('2026.10.0')), (ALARM, RADAR))
        self.assertEqual(to_know(entries(TEXT), image('2026.9.1'), image('2026.10.0')), (RADAR,))
        self.assertEqual(to_know(entries(TEXT), image('2026.9.0'), image('2026.9.1')), (ALARM,))
        self.assertEqual(to_know(entries(TEXT), image('2026.10.0'), image('2026.10.0')), ())

    def test_a_version_that_is_not_a_release_bounds_nothing(self):
        self.assertEqual(to_know(entries(TEXT), image('2026.9.1-snapshot'), image('2026.9.1')), (ALARM,))
        self.assertEqual(to_know(entries(TEXT), image('2026.9.1'), image('2026.10.1-snapshot')), (RADAR,))


if __name__ == '__main__':
    unittest.main()
