import unittest

from cute_display_installer.config.airports import around, km_between, render

NOTRE_DAME = (48.8530, 2.3499)


def airport(ident, kind, lat, lon, name, icao=''):
    return {'ident': ident, 'type': kind, 'latitude_deg': str(lat), 'longitude_deg': str(lon), 'name': name,
            'icao_code': icao, 'gps_code': ''}


class Airports(unittest.TestCase):
    def test_orly_is_about_fourteen_kilometres_from_notre_dame(self):
        self.assertAlmostEqual(km_between(*NOTRE_DAME, 48.7233, 2.3794), 14.6, delta=0.3)

    def test_airports_within_reach_come_nearest_first_with_their_best_code(self):
        rows = [
            airport('LFPG', 'large_airport', 49.0097, 2.5479, 'Paris Charles de Gaulle Airport', icao='LFPG'),
            airport('LFPO', 'large_airport', 48.7233, 2.3794, 'Paris-Orly Airport', icao='LFPO'),
            airport('FR-0001', 'heliport', 48.83, 2.27, 'Issy heliport'),
            airport('LFML', 'large_airport', 43.4393, 5.2214, 'Marseille Provence Airport', icao='LFML'),
            airport('FR-0002', 'small_airport', 48.9, 2.6, 'A field without an ICAO code'),
        ]
        self.assertEqual([airport.line for airport in around(*NOTRE_DAME, rows)], [
            'LFPO 48.7233 2.3794 Paris-Orly Airport',
            'FR-0002 48.9000 2.6000 A field without an ICAO code',
            'LFPG 49.0097 2.5479 Paris Charles de Gaulle Airport',
        ])

    def test_the_file_says_how_many_it_holds(self):
        orly = around(*NOTRE_DAME, [airport('LFPO', 'large_airport', 48.7233, 2.3794, 'Paris-Orly Airport', icao='LFPO')])
        self.assertTrue(orly[0].is_large)
        self.assertEqual(render(orly),
                         '# 1 airports within 100 km, from OurAirports\nLFPO 48.7233 2.3794 Paris-Orly Airport\n')


if __name__ == '__main__':
    unittest.main()
