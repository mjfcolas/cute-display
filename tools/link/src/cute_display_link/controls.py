"""The device's buttons and wheel, through its console."""
from .console import REPLY_TIMEOUT_S, Request

BUTTONS = ('wheel', 'yellow', 'long')


def tap(link, button):
    Request(link, f'tap {button}').expect('ok')


def hold_until_released(link, milliseconds, buttons):
    Request(link, f'hold {milliseconds} {" ".join(buttons)}', REPLY_TIMEOUT_S + milliseconds / 1000).expect('ok')


def turn(link, clockwise_detents):
    Request(link, f'turn {clockwise_detents}').expect('ok')
