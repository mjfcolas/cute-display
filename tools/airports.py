#!/usr/bin/env python3
"""Put the airports around the radar's place on the device, for it to draw as dots.

  tools/airports.py                         for the place in the device's cute-display/radar.conf
  tools/airports.py <latitude> <longitude>  only print what would be sent

Airports come from OurAirports (ourairports.com, public domain): airports of every size,
airfields included, within 100 km. The download is kept in ~/.cache/cute-display.
The other end is infrastructure/src/airports_file.rs.
"""
import csv
import math
import os
import sys
import tempfile
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import sd  # noqa: E402

SOURCE = 'https://davidmegginson.github.io/ourairports-data/airports.csv'
CACHE = os.path.expanduser('~/.cache/cute-display/airports.csv')
KINDS = {'large_airport', 'medium_airport', 'small_airport'}
RADIUS_KM = 100
DEVICE_FILE = 'cute-display/airports.conf'
PLACE_FILE = 'cute-display/radar.conf'


def ourairports():
    if not os.path.exists(CACHE):
        os.makedirs(os.path.dirname(CACHE), exist_ok=True)
        print(f'downloading {SOURCE}', file=sys.stderr)
        urllib.request.urlretrieve(SOURCE, CACHE + '.part')
        os.replace(CACHE + '.part', CACHE)
    with open(CACHE, newline='', encoding='utf-8') as f:
        yield from csv.DictReader(f)


def distance_km(lat1, lon1, lat2, lon2):
    p1, p2 = math.radians(lat1), math.radians(lat2)
    dp, dl = p2 - p1, math.radians(lon2 - lon1)
    a = math.sin(dp / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(dl / 2) ** 2
    return 6371 * 2 * math.asin(math.sqrt(a))


def around(latitude, longitude):
    found = []
    for row in ourairports():
        if row['type'] not in KINDS:
            continue
        lat, lon = float(row['latitude_deg']), float(row['longitude_deg'])
        km = distance_km(latitude, longitude, lat, lon)
        if km <= RADIUS_KM:
            code = row['icao_code'] or row['gps_code'] or row['ident']
            found.append((km, f'{code} {lat:.4f} {lon:.4f} {row["name"]}'))
    return [line for _, line in sorted(found)]


def place_on_device(link):
    with tempfile.NamedTemporaryFile(suffix='.conf', delete=False) as tmp:
        path = tmp.name
    try:
        sd.get(link, PLACE_FILE, path)
        with open(path, encoding='utf-8') as f:
            values = {k.strip(): v.strip() for k, v in (line.split('=', 1) for line in f if '=' in line)}
    finally:
        os.unlink(path)
    return float(values['latitude']), float(values['longitude'])


def main():
    args = sys.argv[1:]
    if args and len(args) != 2:
        sys.exit(__doc__)
    if args:
        latitude, longitude = float(args[0]), float(args[1])
        print('\n'.join(around(latitude, longitude)))
        return

    with sd.open_port() as link:
        try:
            latitude, longitude = place_on_device(link)
        except (sd.Refused, KeyError, ValueError) as why:
            sys.exit(f'no place in {PLACE_FILE} on the device ({why}); put it there first')
        lines = around(latitude, longitude)
        with tempfile.NamedTemporaryFile('w', suffix='.conf', delete=False, encoding='utf-8') as tmp:
            tmp.write(f'# {len(lines)} airports within {RADIUS_KM} km, from OurAirports\n')
            tmp.write(''.join(f'{line}\n' for line in lines))
        try:
            sd.put(link, tmp.name, DEVICE_FILE)
        except sd.Refused as why:
            sys.exit(f'put: {why}')
        finally:
            os.unlink(tmp.name)
    print(f'{len(lines)} airports within {RADIUS_KM} km of {latitude}, {longitude}')


if __name__ == '__main__':
    main()
