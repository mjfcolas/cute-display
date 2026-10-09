"""The setup's files on the card, through the maintenance console."""
from cute_display_link import card
from ..config import habity_sounds


class _Listings:
    """What each directory contains, read once; nothing for a directory that the card does
    not have."""

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
    """The text of each path, or None for a file that the card does not have."""
    entries = _Listings(link)

    def present(path):
        directory, _, name = path.rpartition('/')
        return any(not e.is_directory and e.name == name for e in entries(directory))

    return {path: card.read_file(link, path).decode(errors='replace') if present(path) else None for path in paths}


def habity_sounds_missing(link):
    """Habity's sounds that each app has no copy of, by app, by Habity's names."""
    entries = _Listings(link)
    missing = {}
    for sounds in habity_sounds.EVERY:
        ours = {e.name for e in entries(sounds.directory)}
        missing[sounds.app] = [e.name for e in entries(sounds.habity_directory)
                               if not e.is_directory and habity_sounds.is_playable(e.name) and sounds.copy_name(e.name) not in ours]
    return missing


def write(link, files, copies):
    """Writes `files` as {path: text}, then makes `copies` as {source: destination} on the
    device."""
    for path, text in files.items():
        card.write_file(link, path, text.encode())
    for source, destination in copies.items():
        card.copy(link, source, destination)
