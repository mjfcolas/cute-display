"""What the device shows and says on its glass, and does with its lights and its speaker,
through its console."""
import base64
from dataclasses import dataclass

from .console import Request
from .frame import Frame


@dataclass(frozen=True)
class Screen:
    times_shown: int
    frame: Frame


def screen(link):
    ink = bytearray()
    for kind, rest in Request(link, 'screen').replies():
        if kind == 'data':
            ink += base64.b64decode(rest)
        elif kind == 'ok':
            return Screen(int(rest), Frame(bytes(ink)))


@dataclass(frozen=True)
class Said:
    times_shown: int
    lines: tuple[str, ...]


def said(link):
    lines = []
    for kind, rest in Request(link, 'describe').replies():
        if kind == 'text':
            lines.append(rest)
        elif kind == 'ok':
            return Said(int(rest), tuple(lines))


def lights(link):
    """The front light's and the reading lamp's brightness, in percent."""
    front, lamp = Request(link, 'lights').expect('ok').split()
    return int(front), int(lamp)


def is_playing(link):
    return Request(link, 'sound').expect('ok') == 'playing'

