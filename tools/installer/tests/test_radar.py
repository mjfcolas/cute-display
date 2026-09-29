import unittest

from cute_display_installer.config import general
from cute_display_installer.config.airports import FILE as AIRPORTS_FILE
from cute_display_installer.config.place import Place
from cute_display_installer.config.radar import labels, put_airports, render
from cute_display_link_testing.fake_card import Card

ORLY = {'ident': 'LFPO', 'type': 'large_airport', 'latitude_deg': '48.7233', 'longitude_deg': '2.3794',
        'name': 'Paris-Orly Airport', 'icao_code': 'LFPO', 'gps_code': ''}


class Radar(unittest.TestCase):
    def test_the_airports_around_the_device_go_onto_the_card(self):
        card = Card({general.FILE: b'latitude = 48.8530\nlongitude = 2.3499\n'})
        found, at = put_airports(card, [ORLY])
        self.assertEqual(([airport.line for airport in found], at),
                         (['LFPO 48.7233 2.3794 Paris-Orly Airport'], Place('', 48.853, 2.3499)))
        self.assertEqual(card.files[AIRPORTS_FILE],
                         b'# 1 airports within 100 km, from OurAirports\nLFPO 48.7233 2.3794 Paris-Orly Airport\n')

    def test_without_a_place_there_are_no_airports_to_put(self):
        with self.assertRaises(general.NoPlace):
            put_airports(Card({general.FILE: b'time_zone = UTC0\n'}), [ORLY])

    def test_labels_are_listed_codes(self):
        self.assertEqual(labels('airport_labels = LFPG, lfpo,,LFPB \n'), ['LFPG', 'LFPO', 'LFPB'])
        self.assertEqual(labels('airport_labels = LFPG LFPO\n'), ['LFPG', 'LFPO'])
        self.assertEqual(labels(''), [])

    def test_what_is_rendered_reads_back(self):
        text = render(['LFPG', 'LFPO'])
        self.assertEqual(text, 'airport_labels = LFPG, LFPO\n')
        self.assertEqual(labels(text), ['LFPG', 'LFPO'])


if __name__ == '__main__':
    unittest.main()
