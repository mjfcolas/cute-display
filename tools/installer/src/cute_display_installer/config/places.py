"""Places found by their name, from Open-Meteo's geocoding (open-meteo.com), the
weather's own source."""
import json
import urllib.parse
from dataclasses import dataclass

from .place import Place

SEARCH = 'https://geocoding-api.open-meteo.com/v1/search'
RESULTS = 10


@dataclass(frozen=True)
class Found:
    place: Place
    region: str
    time_zone: str

    def __str__(self):
        return f'{self.place.name}, {self.region}' if self.region else self.place.name


def search(name, fetch):
    query = urllib.parse.urlencode({'name': name, 'count': RESULTS, 'language': 'en', 'format': 'json'})
    results = json.loads(fetch(f'{SEARCH}?{query}')).get('results', [])
    return [Found(place=Place(r['name'], r['latitude'], r['longitude']),
                  region=', '.join(part for part in (r.get('admin1'), r.get('country')) if part),
                  time_zone=r.get('timezone', ''))
            for r in results]
