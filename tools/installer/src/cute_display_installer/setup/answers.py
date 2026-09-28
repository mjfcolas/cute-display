"""What the setup asks, from what the card already holds to the files it writes back."""
from dataclasses import dataclass, field

from ..config import airports, clock, place, radar, wifi
from ..config.place import Place
from ..config.wifi import Wifi

CARD_FILES = [wifi.FILE, place.WEATHER_FILE, radar.FILE, clock.FILE]
# The ESP-IDF Wi-Fi driver's limits (src/engine/drivers/src/esp_wifi.rs).
SSID_BYTES, PASSWORD_BYTES = 32, 64


@dataclass
class Answers:
    wifi: Wifi | None = None
    place: Place | None = None
    time_zone: str | None = None
    labels: list[str] = field(default_factory=list)

    @classmethod
    def from_card(cls, texts):
        """From the files CARD_FILES, each one's text or None when the card lacks it."""
        def text(path):
            return texts.get(path) or ''

        return cls(
            wifi=wifi.read(text(wifi.FILE)),
            place=place.read(text(place.WEATHER_FILE)) or place.read(text(radar.FILE)),
            time_zone=clock.read(text(clock.FILE)),
            labels=radar.labels(text(radar.FILE)),
        )

    def files(self, airports_found):
        """Every file the setup writes, `airports_found` around the place among them."""
        return {
            wifi.FILE: wifi.render(self.wifi),
            place.WEATHER_FILE: place.render(self.place),
            radar.FILE: radar.render(self.place, self.labels),
            airports.FILE: airports.render(airports_found),
            clock.FILE: clock.render(self.time_zone),
        }


def wifi_problem(ssid, password):
    """Why the device could not join this network as typed; None when it can."""
    if not ssid:
        return 'The network needs a name.'
    if len(ssid.encode()) > SSID_BYTES:
        return f'A network name holds {SSID_BYTES} bytes at most.'
    if len(password.encode()) > PASSWORD_BYTES:
        return f'A password holds {PASSWORD_BYTES} bytes at most.'
    if ssid != ssid.strip() or password != password.strip():
        return 'A name or password starting or ending with a space cannot be kept: the device trims them.'
    return None
