import unittest

from cute_display_link_testing.fake_card import FakeCard
from cute_display_link_testing.fake_console import FakeConsole

from cute_display_e2e.device import Device, NotTheTestImage

CLOCKS_OWN = {'cute-display/wifi.conf': b'ssid = Home\n', 'cute-display/settings.conf': b'backlight = 5s\n'}


def device(card):
    return Device(FakeConsole(card))


class OwnDirectory(unittest.TestCase):
    def test_what_a_scenario_left_goes_and_the_clocks_wifi_comes_the_clocks_own_staying(self):
        card = FakeCard({**CLOCKS_OWN, 'cute-display-e2e/apps/alarm/alarm.conf': b'enabled = yes\n',
                         'cute-display-e2e/settings.conf': b'reading_lamp = 10%\n'}, own_directory='cute-display-e2e')
        device(card).empty_own_directory_but_the_clocks_wifi()
        self.assertEqual(card.files, {**CLOCKS_OWN, 'cute-display-e2e/wifi.conf': b'ssid = Home\n'})

    def test_a_directory_not_there_yet_is_empty(self):
        card = FakeCard(CLOCKS_OWN, own_directory='cute-display-e2e')
        device(card).empty_own_directory_but_the_clocks_wifi()
        self.assertEqual(card.files['cute-display-e2e/wifi.conf'], b'ssid = Home\n')

    def test_on_the_app_image_it_says_so_and_changes_nothing(self):
        files = {**CLOCKS_OWN, 'cute-display-e2e/e2e.conf': b'address = 192.168.0.13\n'}
        card = FakeCard(files)
        with self.assertRaisesRegex(NotTheTestImage, 'just fw e2e'):
            device(card).empty_own_directory_but_the_clocks_wifi()
        self.assertEqual(card.files, files)

    def test_files_are_put_by_their_paths_in_its_directory(self):
        card = FakeCard({}, own_directory='cute-display-e2e')
        device(card).put({'apps/timer/timer.conf': b'countdown = 1\n'})
        self.assertEqual(card.files, {'cute-display-e2e/apps/timer/timer.conf': b'countdown = 1\n'})


if __name__ == '__main__':
    unittest.main()
