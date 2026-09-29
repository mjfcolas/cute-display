"""The setup's files on the card, through the maintenance console."""
from cute_display_link import card
from ..config import ringtones


class _Listings:
    """What each directory holds, read once; nothing for a directory the card lacks."""

    def __init__(self, link):
        self.link = link
        self.listed = {}

    def __call__(self, directory):
        if directory not in self.listed:
            parent, _, name = directory.rpartition('/')
            there = not directory or any(e.is_directory and e.name == name for e in self(parent))
            self.listed[directory] = card.entries(self.link, directory) if there else []
        return self.listed[directory]


def read_texts(link, paths):
    """Each path's text, or None for a file the card does not hold."""
    entries = _Listings(link)

    def present(path):
        directory, _, name = path.rpartition('/')
        return any(not e.is_directory and e.name == name for e in entries(directory))

    return {path: card.read_file(link, path).decode(errors='replace') if present(path) else None for path in paths}


def habity_ringtones_missing(link):
    """Habity's alarm ringtones the alarm has no copy of, by name."""
    entries = _Listings(link)
    ours = {e.name for e in entries(ringtones.DIRECTORY)}
    return [e.name for e in entries(ringtones.HABITY_DIRECTORY)
            if not e.is_directory and ringtones.is_ringtone(e.name) and e.name not in ours]


def write(link, files, copies):
    """`files` as {path: text}, then `copies` as {source: destination}, made on the device."""
    for path, text in files.items():
        card.write_file(link, path, text.encode())
    for source, destination in copies.items():
        card.copy(link, source, destination)
