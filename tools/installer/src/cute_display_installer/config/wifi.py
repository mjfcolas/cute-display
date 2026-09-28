"""wifi.conf: the network every app that goes online joins (src/engine/infrastructure/src/internet.rs)."""
from dataclasses import dataclass

from . import conf_text

FILE = 'cute-display/wifi.conf'


@dataclass(frozen=True)
class Wifi:
    ssid: str
    password: str


def read(text):
    values = conf_text.parse(text)
    return Wifi(values['ssid'], values.get('password', '')) if values.get('ssid') else None


def render(wifi):
    return conf_text.render([('ssid', wifi.ssid), ('password', wifi.password)])
