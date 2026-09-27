import unittest

from cute_display_installer.config.airports import FILE as AIRPORTS_FILE
from cute_display_installer.config.place import Place
from cute_display_installer.config.radar import FILE as RADAR_FILE
from cute_display_installer.config.radar import NoPlace, labels, place_of, put_airports, render
from fake_card import Card

ORLY = {'ident': 'LFPO', 'type': 'large_airport', 'latitude_deg': '48.7233', 'longitude_deg': '2.3794',
        'name': 'Paris-Orly Airport', 'icao_code': 'LFPO', 'gps_code': ''}


class Radar(unittest.TestCase):
    def test_the_place_is_read_from_radar_conf(self):
        self.assertEqual(place_of('place = Notre-Dame\nlatitude = 48.8530\nlongitude = 2.3499\n'), Place('Notre-Dame', 48.853, 2.3499))

    def test_a_missing_or_unreadable_place_is_said(self):
        for text in ('place = Notre-Dame\n', 'latitude = north\nlongitude = 2.3\n'):
            with self.assertRaisesRegex(NoPlace, 'No place'):
                place_of(text)

    def test_the_airports_around_the_place_go_onto_the_card(self):
        card = Card({RADAR_FILE: b'latitude = 48.8530\nlongitude = 2.3499\n'})
        found, at = put_airports(card, [ORLY])
        self.assertEqual(([airport.line for airport in found], at),
                         (['LFPO 48.7233 2.3794 Paris-Orly Airport'], Place('', 48.853, 2.3499)))
        self.assertEqual(card.files[AIRPORTS_FILE],
                         b'# 1 airports within 100 km, from OurAirports\nLFPO 48.7233 2.3794 Paris-Orly Airport\n')


    def test_labels_are_listed_codes(self):
        self.assertEqual(labels('airport_labels = LFPG, lfpo,,LFPB \n'), ['LFPG', 'LFPO', 'LFPB'])
        self.assertEqual(labels('airport_labels = LFPG LFPO\n'), ['LFPG', 'LFPO'])
        self.assertEqual(labels('place = Notre-Dame\n'), [])

    def test_what_is_rendered_reads_back(self):
        text = render(Place('Notre-Dame', 48.853, 2.3499), ['LFPG', 'LFPO'])
        self.assertEqual(text, 'place = Notre-Dame\nlatitude = 48.8530\nlongitude = 2.3499\nairport_labels = LFPG, LFPO\n')
        self.assertEqual((place_of(text), labels(text)), (Place('Notre-Dame', 48.853, 2.3499), ['LFPG', 'LFPO']))


if __name__ == '__main__':
    unittest.main()
