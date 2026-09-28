"""radar.conf: the radar's place and which of its airports carry their code; the
airports themselves go beside it, in airports.conf."""
import re

from ..card import console
from . import airports, conf_text, place

FILE = 'cute-display/radar.conf'


class NoPlace(Exception):
    pass


def place_of(radar_conf):
    found = place.read(radar_conf)
    if found is None:
        raise NoPlace(f'No place in {FILE} on the device: put it there first.')
    return found


def labels(radar_conf):
    listed = conf_text.parse(radar_conf).get('airport_labels', '')
    # As src/apps/radar/src/infrastructure/airports_file.rs splits them: on commas and spaces.
    return [code.upper() for code in re.split(r'[,\s]+', listed) if code]


def render(at, labelled):
    return conf_text.render(place.pairs(at) + [('airport_labels', ', '.join(labelled))])


def put_airports(link, rows):
    """Writes airports.conf for the place in radar.conf; the airports written, and the place."""
    at = place_of(console.read_file(link, FILE).decode(errors='replace'))
    found = airports.around(at.latitude, at.longitude, rows)
    console.write_file(link, airports.FILE, airports.render(found).encode())
    return found, at
