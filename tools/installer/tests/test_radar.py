import unittest

from cute_display_installer.config.radar import AIRPORTS_FILE, RADAR_FILE, NoPlace, place, put_airports
from fake_card import Card

ORLY = {'ident': 'LFPO', 'type': 'large_airport', 'latitude_deg': '48.7233', 'longitude_deg': '2.3794',
        'name': 'Paris-Orly Airport', 'icao_code': 'LFPO', 'gps_code': ''}


class Radar(unittest.TestCase):
    def test_the_place_is_read_from_radar_conf(self):
        self.assertEqual(place('place = Notre-Dame\nlatitude = 48.8530\nlongitude = 2.3499\n'), (48.853, 2.3499))

    def test_a_missing_or_unreadable_place_is_said(self):
        for text in ('place = Notre-Dame\n', 'latitude = north\nlongitude = 2.3\n'):
            with self.assertRaisesRegex(NoPlace, 'No place'):
                place(text)

    def test_the_airports_around_the_place_go_onto_the_card(self):
        card = Card({RADAR_FILE: b'latitude = 48.8530\nlongitude = 2.3499\n'})
        lines, at = put_airports(card, [ORLY])
        self.assertEqual((lines, at), (['LFPO 48.7233 2.3794 Paris-Orly Airport'], (48.853, 2.3499)))
        self.assertEqual(card.files[AIRPORTS_FILE],
                         b'# 1 airports within 100 km, from OurAirports\nLFPO 48.7233 2.3794 Paris-Orly Airport\n')


if __name__ == '__main__':
    unittest.main()
