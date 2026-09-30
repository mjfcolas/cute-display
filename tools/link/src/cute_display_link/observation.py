"""What the device does with its lights and its speaker, through its console."""
from .console import Request


def lights(link):
    """The front light's and the reading lamp's brightness, in percent."""
    front, lamp = Request(link, 'lights').expect('ok').split()
    return int(front), int(lamp)


def is_playing(link):
    return Request(link, 'sound').expect('ok') == 'playing'

