"""clock.conf: the time zone, as a POSIX rule for the device
(src/infrastructure/src/time_zone_file.rs) and by its name for the installer."""
from . import conf_text, time_zone

FILE = 'cute-display/clock.conf'


def read(text):
    """The zone's name, when the installer wrote it."""
    return conf_text.parse(text).get('time_zone_name') or None


def render(zone_name):
    return conf_text.render([('time_zone', time_zone.posix_of(zone_name)), ('time_zone_name', zone_name)])
