"""What the setup offers about Cute Display, from what the clock holds and the latest
release."""
from dataclasses import dataclass
from enum import Enum

from .. import updating
from ..flash.layout import AppImage, Placement, Slot


class Offering(Enum):
    INSTALL = 'install'
    UPDATE = 'update'
    START = 'start'
    NOTHING = 'nothing'


@dataclass(frozen=True)
class Offer:
    offering: Offering
    latest: AppImage
    installed: AppImage | None = None
    slot: Slot | None = None
    placement: Placement | None = None
    to_know: tuple[updating.Entry, ...] = ()


def offer(unit, latest, updating_entries=()):
    """`unit` the clock as read, holding Cute Display or able to take it; `latest` the
    latest release's image, `updating_entries` its UPDATING.md's."""
    slot = unit.ours()
    if slot is None:
        return Offer(Offering.INSTALL, latest, placement=unit.placement())
    installed = unit.slots[slot]
    newer = installed.release is None or latest.release is None or latest.release > installed.release
    if newer and not unit.install_refusals():
        return Offer(Offering.UPDATE, latest, installed, slot, unit.placement(),
                     updating.to_know(updating_entries, installed, latest))
    if unit.booting != slot:
        return Offer(Offering.START, latest, installed, slot)
    return Offer(Offering.NOTHING, latest, installed, slot)
