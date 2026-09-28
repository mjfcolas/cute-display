"""general.conf: what every app shares, where the device is and its time zone, and which
apps run, as src/engine/infrastructure/src/general_file.rs reads it; the zone also by
its name, for the installer."""
import re

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


def apps_of(text):
    """The apps chosen, by name; None when the file does not say."""
    listed = conf_text.parse(text).get('apps')
    return None if listed is None else [name for name in re.split(r'[,\s]+', listed) if name]


def render(at, zone_name, apps):
    zone = [('time_zone', time_zone.posix_of(zone_name)), ('time_zone_name', zone_name)]
    return conf_text.render(place.pairs(at) + zone + [('apps', ', '.join(apps))])
