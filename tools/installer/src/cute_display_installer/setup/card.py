"""The setup's files on the card, through the maintenance console."""
from ..card import console


def read_texts(link, paths):
    """Each path's text, or None for a file the card does not hold."""
    listings = {}

    def entries(directory):
        """What `directory` holds; nothing when the card lacks it."""
        if directory not in listings:
            parent, _, name = directory.rpartition('/')
            there = not directory or any(e.is_directory and e.name == name for e in entries(parent))
            listings[directory] = console.entries(link, directory) if there else []
        return listings[directory]

    def present(path):
        directory, _, name = path.rpartition('/')
        return any(not e.is_directory and e.name == name for e in entries(directory))

    return {path: console.read_file(link, path).decode(errors='replace') if present(path) else None for path in paths}


def write(link, files):
    for path, text in files.items():
        console.write_file(link, path, text.encode())
