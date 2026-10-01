"""Versions, the app images' and the installer's, as numbers to compare."""
from importlib import metadata

_DISTRIBUTION = 'cute-display-installer'


def release(version):
    """The version as numbers to compare, `1.1.2` > `1.1.1`; None when it does not read so,
    as a snapshot or a build between releases does (`-snapshot`, `-4-gabc1234`, `-dirty`,
    `.dev3+gabc1234`)."""
    parts = version.split('.')
    return tuple(int(part) for part in parts) if all(part.isdigit() for part in parts) else None


def this_installer_version():
    return metadata.version(_DISTRIBUTION)
