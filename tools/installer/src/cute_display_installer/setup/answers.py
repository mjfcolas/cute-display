"""What the setup asks, from what the card already holds to the files it writes back."""
from dataclasses import dataclass, field

from ..config import airports, apps, general, place, radar, ringtones, wifi
from ..config.place import Place
from ..config.wifi import Wifi

CARD_FILES = [wifi.FILE, general.FILE, radar.FILE]
# The ESP-IDF Wi-Fi driver's limits (src/engine/drivers/src/esp_wifi.rs).
SSID_BYTES, PASSWORD_BYTES = 32, 64


@dataclass
class Answers:
    wifi: Wifi | None = None
    place: Place | None = None
    time_zone: str | None = None
    labels: list[str] = field(default_factory=list)
    apps: list[str] = field(default_factory=lambda: list(apps.TITLES))
    habity_ringtones: list[str] = field(default_factory=list)
    copy_ringtones: bool = False

    @classmethod
    def from_card(cls, texts, habity_ringtones=()):
        """From the files CARD_FILES, each one's text or None when the card lacks it, and
        Habity's alarm ringtones the alarm has no copy of."""
        def text(path):
            return texts.get(path) or ''

        chosen = general.apps_of(text(general.FILE))
        return cls(
            wifi=wifi.read(text(wifi.FILE)),
            place=place.read(text(general.FILE)),
            time_zone=general.time_zone_of(text(general.FILE)),
            labels=radar.labels(text(radar.FILE)),
            apps=list(apps.TITLES) if chosen is None else [name for name in chosen if name in apps.TITLES],
            habity_ringtones=list(habity_ringtones),
        )

    def offers_ringtones(self):
        """Whether the alarm runs and Habity has ringtones it has no copy of."""
        return 'alarm' in self.apps and bool(self.habity_ringtones)

    def copies(self):
        """Every file the setup copies on the card, as {source: destination}."""
        return ringtones.copies(self.habity_ringtones) if self.copy_ringtones and self.offers_ringtones() else {}

    def files(self, airports_found):
        """Every file the setup writes: the radar's too when it is chosen, `airports_found`
        around the place among them."""
        files = {
            wifi.FILE: wifi.render(self.wifi),
            general.FILE: general.render(self.place, self.time_zone, self.apps),
        }
        if 'radar' in self.apps:
            files[radar.FILE] = radar.render(self.labels)
            files[airports.FILE] = airports.render(airports_found)
        return files


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
