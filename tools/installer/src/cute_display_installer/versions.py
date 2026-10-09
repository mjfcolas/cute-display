"""Versions of the app images and of the installer, as numbers to compare."""
from importlib import metadata

_DISTRIBUTION = 'cute-display-installer'


def release(version):
    """The version as numbers to compare, `1.1.2` > `1.1.1`; None when it cannot be read
    that way, as for a snapshot or a build between releases (`-snapshot`, `-4-gabc1234`,
    `-dirty`, `.dev3+gabc1234`)."""
    parts = version.split('.')
    return tuple(int(part) for part in parts) if all(part.isdigit() for part in parts) else None


def this_installer_version():
    return metadata.version(_DISTRIBUTION)
