"""The radar's radar.conf: which of its airports carry their code; the airports
themselves go beside it, in airports.conf."""
import re

from ..card import console
from . import airports, conf_text, general

FILE = 'cute-display/apps/radar/radar.conf'


def labels(radar_conf):
    listed = conf_text.parse(radar_conf).get('airport_labels', '')
    # As src/apps/radar/src/infrastructure/airports_file.rs splits them: on commas and spaces.
    return [code.upper() for code in re.split(r'[,\s]+', listed) if code]


def render(labelled):
    return conf_text.render([('airport_labels', ', '.join(labelled))])


def put_airports(link, rows):
    """Writes airports.conf for the place in general.conf; the airports written, and the place."""
    at = general.place_of(console.read_file(link, general.FILE).decode(errors='replace'))
    found = airports.around(at.latitude, at.longitude, rows)
    console.write_file(link, airports.FILE, airports.render(found).encode())
    return found, at
