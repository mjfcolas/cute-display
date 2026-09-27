"""The setup's files on the card, through the maintenance console."""
from ..card import console


class Unreachable(Exception):
    """The card could not be read or written: unplugged, busy, or not answering."""

DIRECTORY = 'cute-display'


def read_texts(link, paths):
    """Each path's text, or None for a file the card does not hold."""
    if not any(e.is_directory and e.name == DIRECTORY for e in console.entries(link)):
        return dict.fromkeys(paths)
    present = {f'{DIRECTORY}/{e.name}' for e in console.entries(link, DIRECTORY) if not e.is_directory}
    return {path: console.read_file(link, path).decode(errors='replace') if path in present else None
            for path in paths}


def write(link, files):
    for path, text in files.items():
        console.write_file(link, path, text.encode())
