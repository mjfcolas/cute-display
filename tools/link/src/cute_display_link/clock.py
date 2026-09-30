"""The device's RTC, through its console."""
from datetime import datetime, timezone

from .console import Request


def read(link):
    return datetime.fromtimestamp(int(Request(link, 'clock').expect('ok')), timezone.utc)


def set_to(link, when):
    """`when` knows its time zone."""
    Request(link, f'clock set {int(when.timestamp())}').expect('ok')
