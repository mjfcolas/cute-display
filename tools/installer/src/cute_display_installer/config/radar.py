"""The radar's place, in radar.conf on the card, and the airports put beside it."""
from ..card import console
from . import airports, conf_text

RADAR_FILE = 'cute-display/radar.conf'
AIRPORTS_FILE = 'cute-display/airports.conf'


class NoPlace(Exception):
    pass


def place(radar_conf):
    """Latitude and longitude from radar.conf's text."""
    values = conf_text.parse(radar_conf)
    try:
        return float(values['latitude']), float(values['longitude'])
    except (KeyError, ValueError):
        raise NoPlace(f'No place in {RADAR_FILE} on the device: put it there first.') from None


def put_airports(link, rows):
    """Writes airports.conf for the place in radar.conf; the lines written, and the place."""
    latitude, longitude = place(console.read_file(link, RADAR_FILE).decode(errors='replace'))
    lines = airports.around(latitude, longitude, rows)
    console.write_file(link, AIRPORTS_FILE, airports.render(lines).encode())
    return lines, (latitude, longitude)
