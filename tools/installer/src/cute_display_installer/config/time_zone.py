"""Time zones by their IANA name, turned into the POSIX rule the device keeps: the last
line of the zone's TZif file (RFC 8536), from the tzdata package."""
from importlib.resources import files
from zoneinfo import available_timezones

import tzlocal


class UnknownZone(Exception):
    pass


def posix_of(name):
    try:
        tzif = files('tzdata.zoneinfo').joinpath(*name.split('/')).read_bytes()
    except (FileNotFoundError, IsADirectoryError, ValueError):
        raise UnknownZone(f'no time zone named {name}') from None
    return tzif.rstrip(b'\n').rsplit(b'\n', 1)[-1].decode()


def is_known(name):
    try:
        posix_of(name)
    except UnknownZone:
        return False
    return True


def names():
    return sorted(available_timezones())


def of_this_computer():
    return tzlocal.get_localzone_name()
