"""The airports the radar draws around its place, from OurAirports (ourairports.com,
public domain), as src/apps/radar/src/infrastructure/airports_file.rs reads them."""
import csv
import math
import os
from dataclasses import dataclass

from ..web import fetch

SOURCE = 'https://davidmegginson.github.io/ourairports-data/airports.csv'
CACHE = os.path.join(os.path.expanduser('~'), '.cache', 'cute-display', 'airports.csv')
KINDS = {'large_airport', 'medium_airport', 'small_airport'}
RADIUS_KM = 100
EARTH_RADIUS_KM = 6371
FILE = 'cute-display/airports.conf'


@dataclass(frozen=True)
class Airport:
    code: str
    latitude: float
    longitude: float
    name: str
    is_large: bool
    distance_km: float

    @property
    def line(self):
        return f'{self.code} {self.latitude:.4f} {self.longitude:.4f} {self.name}'


def ourairports(report):
    """Every airport of the world, downloaded once and kept in CACHE."""
    if not os.path.exists(CACHE):
        report('Downloading the airports of the world from OurAirports, once...')
        os.makedirs(os.path.dirname(CACHE), exist_ok=True)
        with open(CACHE + '.part', 'wb') as f:
            f.write(fetch(SOURCE))
        os.replace(CACHE + '.part', CACHE)
    with open(CACHE, newline='', encoding='utf-8') as f:
        yield from csv.DictReader(f)


def km_between(lat1, lon1, lat2, lon2):
    p1, p2 = math.radians(lat1), math.radians(lat2)
    dp, dl = p2 - p1, math.radians(lon2 - lon1)
    a = math.sin(dp / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(dl / 2) ** 2
    return EARTH_RADIUS_KM * 2 * math.asin(math.sqrt(a))


def around(latitude, longitude, rows):
    """The airports among `rows` within RADIUS_KM, nearest first."""
    found = []
    for row in rows:
        if row['type'] not in KINDS:
            continue
        lat, lon = float(row['latitude_deg']), float(row['longitude_deg'])
        km = km_between(latitude, longitude, lat, lon)
        if km <= RADIUS_KM:
            code = row['icao_code'] or row['gps_code'] or row['ident']
            found.append(Airport(code, lat, lon, row['name'], row['type'] == 'large_airport', km))
    return sorted(found, key=lambda airport: airport.distance_km)


def render(airports):
    return (f'# {len(airports)} airports within {RADIUS_KM} km, from OurAirports\n'
            + ''.join(f'{airport.line}\n' for airport in airports))
