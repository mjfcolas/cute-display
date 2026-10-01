"""What to know before updating, from a release's UPDATING.md: the entries an update from
one version to another goes through."""
import re
from dataclasses import dataclass

from . import versions

FILE = 'UPDATING.md'
_HEADING = re.compile(r'^## (\S+)[ \t]*$', re.MULTILINE)


@dataclass(frozen=True)
class Entry:
    version: str
    text: str

    @property
    def release(self):
        return versions.release(self.version)


def entries(text):
    """Each `## <version>` section, in the file's order; a section whose heading is not a
    release's version is not one."""
    parts = _HEADING.split(text)[1:]
    found = (Entry(version, body.strip()) for version, body in zip(parts[::2], parts[1::2]))
    return tuple(entry for entry in found if entry.release is not None)


def to_know(among, installed, latest):
    """The entries `among` after `installed` up to `latest`, oldest first, with no bound on
    a side whose version is not a release's."""
    since, until = installed.release, latest.release
    return tuple(sorted((entry for entry in among
                         if (since is None or entry.release > since) and (until is None or entry.release <= until)),
                        key=lambda entry: entry.release))
