"""The airports the radar draws around its place, from OurAirports (ourairports.com,
public domain), as src/infrastructure/src/airports_file.rs reads them."""
import csv
import math
import os
import urllib.request

SOURCE = 'https://davidmegginson.github.io/ourairports-data/airports.csv'
CACHE = os.path.join(os.path.expanduser('~'), '.cache', 'cute-display', 'airports.csv')
KINDS = {'large_airport', 'medium_airport', 'small_airport'}
RADIUS_KM = 100
EARTH_RADIUS_KM = 6371


def ourairports(report):
    """Every airport of the world, downloaded once and kept in CACHE."""
    if not os.path.exists(CACHE):
        report('Downloading the airports of the world from OurAirports, once...')
        os.makedirs(os.path.dirname(CACHE), exist_ok=True)
        urllib.request.urlretrieve(SOURCE, CACHE + '.part')
        os.replace(CACHE + '.part', CACHE)
    with open(CACHE, newline='', encoding='utf-8') as f:
        yield from csv.DictReader(f)


def distance_km(lat1, lon1, lat2, lon2):
    p1, p2 = math.radians(lat1), math.radians(lat2)
    dp, dl = p2 - p1, math.radians(lon2 - lon1)
    a = math.sin(dp / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(dl / 2) ** 2
    return EARTH_RADIUS_KM * 2 * math.asin(math.sqrt(a))


def around(latitude, longitude, rows):
    """The lines of airports.conf for the airports among `rows` within RADIUS_KM, nearest
    first."""
    found = []
    for row in rows:
        if row['type'] not in KINDS:
            continue
        lat, lon = float(row['latitude_deg']), float(row['longitude_deg'])
        km = distance_km(latitude, longitude, lat, lon)
        if km <= RADIUS_KM:
            code = row['icao_code'] or row['gps_code'] or row['ident']
            found.append((km, f'{code} {lat:.4f} {lon:.4f} {row["name"]}'))
    return [line for _, line in sorted(found)]


def render(lines):
    return f'# {len(lines)} airports within {RADIUS_KM} km, from OurAirports\n' + ''.join(f'{line}\n' for line in lines)
