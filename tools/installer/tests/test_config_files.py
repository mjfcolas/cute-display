import json
import pathlib
import unittest

from cute_display_installer.config import general, place, places, time_zone, wifi
from cute_display_installer.config.place import Place

# What src/engine/domain/tests/tzdata_rules.rs parses as the device does.
DEVICE_RULES = pathlib.Path(__file__).resolve().parents[3] / 'src' / 'engine' / 'domain' / 'tests' / 'tzdata_rules.txt'


class Wifi(unittest.TestCase):
    def test_what_is_rendered_reads_back(self):
        home = wifi.Wifi('Home', 'a=b c')
        self.assertEqual(wifi.read(wifi.render(home)), home)

    def test_without_a_network_name_there_is_no_wifi(self):
        self.assertIsNone(wifi.read('password = secret\n'))
        self.assertEqual(wifi.read('ssid = Open\n'), wifi.Wifi('Open', ''))


class Places(unittest.TestCase):
    def test_a_place_needs_both_coordinates(self):
        self.assertEqual(place.read('place = Paris\nlatitude = 48.85\nlongitude = 2.35\n'), Place('Paris', 48.85, 2.35))
        self.assertIsNone(place.read('place = Paris\nlatitude = 48.85\n'))
        self.assertIsNone(place.read('latitude = north\nlongitude = 2.35\n'))

    def test_places_are_found_by_name_with_their_region_and_time_zone(self):
        answer = {'results': [
            {'name': 'Montreal', 'latitude': 45.50884, 'longitude': -73.58781, 'country': 'Canada',
             'admin1': 'Quebec', 'timezone': 'America/Toronto'},
            {'name': 'Montréal', 'latitude': 43.2, 'longitude': 2.14, 'country': 'France',
             'timezone': 'Europe/Paris'},
        ]}
        asked = []

        def fetch(url):
            asked.append(url)
            return json.dumps(answer).encode()

        found = places.search('Montréal', fetch)
        self.assertIn('name=Montr%C3%A9al', asked[0])
        self.assertEqual([str(f) for f in found], ['Montreal, Quebec, Canada', 'Montréal, France'])
        self.assertEqual((found[0].place, found[0].time_zone), (Place('Montreal', 45.50884, -73.58781), 'America/Toronto'))

    def test_nothing_found_is_an_empty_list(self):
        self.assertEqual(places.search('Nowhere at all', lambda _: b'{"generationtime_ms": 0.1}'), [])


class TimeZones(unittest.TestCase):
    def test_a_zone_becomes_its_posix_rule(self):
        self.assertEqual(time_zone.posix_of('Europe/Paris'), 'CET-1CEST,M3.5.0,M10.5.0/3')
        self.assertEqual(time_zone.posix_of('America/Toronto'), 'EST5EDT,M3.2.0,M11.1.0')
        self.assertEqual(time_zone.posix_of('Asia/Tokyo'), 'JST-9')

    def test_an_unknown_zone_is_said(self):
        for name in ('Europe/Atlantis', 'Europe', ''):
            with self.assertRaises(time_zone.UnknownZone):
                time_zone.posix_of(name)

    def test_the_device_is_tested_on_every_rule_offered(self):
        listed = ''.join(f'{name} {time_zone.posix_of(name)}\n' for name in time_zone.names())
        self.assertIn('Europe/Paris CET-1CEST,M3.5.0,M10.5.0/3\n', listed)
        self.assertEqual(DEVICE_RULES.read_text(), listed,
                         f'tzdata changed: write the new list into {DEVICE_RULES} for the device\'s test')

    def test_general_conf_keeps_the_rule_for_the_device_and_the_name_for_the_installer(self):
        text = general.render(Place('Notre-Dame', 48.853, 2.3499), 'Europe/Paris')
        self.assertEqual(text, 'place = Notre-Dame\nlatitude = 48.8530\nlongitude = 2.3499\n'
                               'time_zone = CET-1CEST,M3.5.0,M10.5.0/3\ntime_zone_name = Europe/Paris\n')
        self.assertEqual((general.place_of(text), general.time_zone_of(text)), (Place('Notre-Dame', 48.853, 2.3499), 'Europe/Paris'))
        self.assertIsNone(general.time_zone_of('time_zone = EST5EDT,M3.2.0,M11.1.0\n'))

    def test_a_missing_or_unreadable_place_is_said(self):
        for text in ('place = Notre-Dame\n', 'latitude = north\nlongitude = 2.3\n'):
            with self.assertRaisesRegex(general.NoPlace, 'No place'):
                general.place_of(text)


if __name__ == '__main__':
    unittest.main()
