"""The clock under test as a person meets it, through its console alone: its controls, what
its glass says, its lights, its speaker, its clock and its card."""
from typing import NamedTuple

from cute_display_link import card, clock, controls, observation
from cute_display_link.console import ConsoleError

from .screen import Screen
from .waiting import until

# The glass refreshes in up to 7 s cold, on the device; what follows a control may take a
# few refreshes.
SCREEN_TIMEOUT_S = 20
# The console's answer before the first frame is shown, src/maintenance/src/console.rs.
NOTHING_SAID = 'nothing said yet'


class Lights(NamedTuple):
    front_percent: int
    reading_lamp_percent: int


class Display:
    def __init__(self, link):
        self.link = link

    def tap(self, button):
        controls.tap(self.link, button)

    def hold(self, milliseconds, *buttons):
        controls.hold_until_released(self.link, milliseconds, buttons)

    def turn(self, clockwise_detents):
        controls.turn(self.link, clockwise_detents)

    def screen(self):
        try:
            return Screen(observation.said(self.link).lines)
        except ConsoleError as error:
            if str(error) != NOTHING_SAID:
                raise
            return Screen(())

    def until_screen(self, condition, what_did_not_happen, timeout_s=SCREEN_TIMEOUT_S):
        """The screen once `condition(screen)` holds; failing, says the last one seen."""
        seen = []

        def holds():
            seen[:] = [self.screen()]
            return seen[0] if condition(seen[0]) else None

        try:
            return until(holds, timeout_s, what_did_not_happen)
        except AssertionError as failed:
            raise AssertionError(f'{failed}; the glass said:\n{seen[0] if seen else "nothing"}') from None

    def until_front(self, app):
        return self.until_screen(lambda screen: screen.front() == app, f'{app} did not come to the front')

    def until_chosen(self, line):
        return self.until_screen(lambda screen: screen.chosen() == line, f'{line} was not chosen')

    def lights(self):
        return Lights(*observation.lights(self.link))

    def is_playing(self):
        return observation.is_playing(self.link)

    def time(self):
        return clock.read(self.link)

    def read_file(self, path):
        return card.read_file(self.link, path)

    def write_file(self, path, contents):
        card.write_file(self.link, path, contents)
