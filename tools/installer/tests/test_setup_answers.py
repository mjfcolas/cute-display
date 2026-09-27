import unittest

from cute_display_installer.config.airports import Airport
from cute_display_installer.config.place import Place
from cute_display_installer.config.wifi import Wifi
from cute_display_installer.setup import card
from cute_display_installer.setup.answers import CARD_FILES, Answers, wifi_problem
from fake_card import Card

NOTRE_DAME = Place('Notre-Dame', 48.853, 2.3499)
ORLY = Airport('LFPO', 48.7233, 2.3794, 'Paris-Orly Airport', True, 14.6)


class FromTheCard(unittest.TestCase):
    def test_a_card_set_up_before_answers_everything(self):
        texts = {
            'cute-display/wifi.conf': 'ssid = Home\npassword = s3cret\n',
            'cute-display/weather.conf': 'place = Notre-Dame\nlatitude = 48.8530\nlongitude = 2.3499\n',
            'cute-display/radar.conf': 'place = Orly\nlatitude = 48.7\nlongitude = 2.4\nairport_labels = LFPO\n',
            'cute-display/clock.conf': 'time_zone = CET-1CEST,M3.5.0,M10.5.0/3\ntime_zone_name = Europe/Paris\n',
        }
        self.assertEqual(Answers.from_card(texts),
                         Answers(Wifi('Home', 's3cret'), NOTRE_DAME, 'Europe/Paris', ['LFPO']))

    def test_the_radar_place_stands_in_for_a_missing_weather_place(self):
        texts = dict.fromkeys(CARD_FILES)
        texts['cute-display/radar.conf'] = 'place = Orly\nlatitude = 48.7\nlongitude = 2.4\n'
        self.assertEqual(Answers.from_card(texts).place, Place('Orly', 48.7, 2.4))

    def test_an_empty_card_answers_nothing(self):
        self.assertEqual(Answers.from_card(dict.fromkeys(CARD_FILES)), Answers())


class ToTheCard(unittest.TestCase):
    def test_every_file_is_written_and_reads_back_the_same(self):
        answers = Answers(Wifi('Home', 's3cret'), NOTRE_DAME, 'Europe/Paris', ['LFPO'])
        files = answers.files([ORLY])
        self.assertEqual(sorted(files), sorted(CARD_FILES + ['cute-display/airports.conf']))
        self.assertEqual(files['cute-display/airports.conf'].splitlines()[1], 'LFPO 48.7233 2.3794 Paris-Orly Airport')
        self.assertEqual(Answers.from_card(files), answers)

    def test_the_card_is_read_then_written_through_the_console(self):
        device = Card({'cute-display/wifi.conf': b'ssid = Home\n', 'sounds/alarm.wav': b''})
        texts = card.read_texts(device, CARD_FILES)
        self.assertEqual(texts['cute-display/wifi.conf'], 'ssid = Home\n')
        self.assertIsNone(texts['cute-display/clock.conf'])
        card.write(device, {'cute-display/clock.conf': 'time_zone = JST-9\n'})
        self.assertEqual(device.files['cute-display/clock.conf'], b'time_zone = JST-9\n')

    def test_a_card_without_our_directory_holds_none_of_the_files(self):
        self.assertEqual(card.read_texts(Card({'sounds/alarm.wav': b''}), CARD_FILES), dict.fromkeys(CARD_FILES))


class WifiAsTyped(unittest.TestCase):
    def test_a_network_the_device_can_join_passes(self):
        self.assertIsNone(wifi_problem('Home', 'correct horse battery staple'))
        self.assertIsNone(wifi_problem('Open', ''))

    def test_what_the_device_could_not_keep_is_said(self):
        self.assertIn('needs a name', wifi_problem('', 'secret'))
        self.assertIn('32 bytes', wifi_problem('é' * 17, ''))
        self.assertIn('64 bytes', wifi_problem('Home', 'x' * 65))
        self.assertIn('trims', wifi_problem('Home', 'secret '))


if __name__ == '__main__':
    unittest.main()
