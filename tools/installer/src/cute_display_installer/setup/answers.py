"""What the setup asks, from what the card already holds to the files it writes back."""
from dataclasses import dataclass, field

from ..config import airports, apps, general, habity_sounds, place, radar, wifi
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
    # Habity's sounds each app lacks, by app, and the apps that are to have theirs copied.
    habity_sounds: dict[str, list[str]] = field(default_factory=dict)
    copied: set[str] = field(default_factory=set)

    @classmethod
    def from_card(cls, texts, habity_sounds=None):
        """From the files CARD_FILES, each one's text or None when the card lacks it, and
        Habity's sounds each app has no copy of, by app."""
        def text(path):
            return texts.get(path) or ''

        chosen = general.apps_of(text(general.FILE))
        return cls(
            wifi=wifi.read(text(wifi.FILE)),
            place=place.read(text(general.FILE)),
            time_zone=general.time_zone_of(text(general.FILE)),
            labels=radar.labels(text(radar.FILE)),
            apps=list(apps.TITLES) if chosen is None else [name for name in chosen if name in apps.TITLES],
            habity_sounds=dict(habity_sounds or {}),
        )

    def offers(self, sounds):
        """Whether the app of Habity's `sounds` runs and Habity has some it has no copy of."""
        return sounds.app in self.apps and bool(self.habity_sounds.get(sounds.app))

    def copied_sounds(self):
        """The HabitySounds copied, each with Habity's names of them."""
        return [(sounds, self.habity_sounds[sounds.app])
                for sounds in habity_sounds.EVERY if sounds.app in self.copied and self.offers(sounds)]

    def copies(self):
        """Every file the setup copies on the card, as {source: destination}."""
        return {source: destination for sounds, names in self.copied_sounds() for source, destination in sounds.copies(names).items()}

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
