"""A place the weather or the radar is about, as src/infrastructure/src/place_file.rs
reads it."""
from dataclasses import dataclass

from . import conf_text

WEATHER_FILE = 'cute-display/weather.conf'


@dataclass(frozen=True)
class Place:
    name: str
    latitude: float
    longitude: float


def read(text):
    values = conf_text.parse(text)
    try:
        return Place(values.get('place', ''), float(values['latitude']), float(values['longitude']))
    except (KeyError, ValueError):
        return None


def pairs(place):
    return [('place', place.name), ('latitude', f'{place.latitude:.4f}'), ('longitude', f'{place.longitude:.4f}')]


def render(place):
    return conf_text.render(pairs(place))
