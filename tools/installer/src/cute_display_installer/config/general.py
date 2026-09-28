"""general.conf: what every app shares, where the device is and its time zone, as
src/engine/infrastructure/src/general_file.rs reads it; the zone also by its name, for
the installer."""
from . import conf_text, place, time_zone

FILE = 'cute-display/general.conf'


class NoPlace(Exception):
    pass


def place_of(text):
    found = place.read(text)
    if found is None:
        raise NoPlace(f'No place in {FILE} on the device: put it there first.')
    return found


def time_zone_of(text):
    """The zone's name, when the installer wrote it."""
    return conf_text.parse(text).get('time_zone_name') or None


def render(at, zone_name):
    return conf_text.render(place.pairs(at) + [('time_zone', time_zone.posix_of(zone_name)), ('time_zone_name', zone_name)])
