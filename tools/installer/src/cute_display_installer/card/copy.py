"""The card, or a directory of it, copied to the computer."""
import os
import time

from .console import entries, read_file


def pull(link, directory, destination, report=print):
    """Every file under `directory`, skipping those already there at the same size, so
    that an interrupted copy resumes."""
    for entry in entries(link, directory):
        path = f'{directory}/{entry.name}' if directory else entry.name
        local = os.path.join(destination, entry.name)
        if entry.is_directory:
            pull(link, path, local, report)
        elif os.path.exists(local) and os.path.getsize(local) == entry.size:
            report(f'{path}: already there')
        else:
            started = time.monotonic()
            contents = read_file(link, path)
            os.makedirs(destination, exist_ok=True)
            with open(local + '.part', 'wb') as f:
                f.write(contents)
            os.replace(local + '.part', local)
            elapsed = max(time.monotonic() - started, 1e-3)
            report(f'{path} ({len(contents)} bytes, {len(contents) / 1024 / elapsed:.0f} KB/s)')
