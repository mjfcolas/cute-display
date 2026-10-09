"""The clock under test as a person uses it, only through its console: its controls, what
its screen says, its lights and its speaker."""
from typing import NamedTuple

from cute_display_link import controls, observation
from cute_display_link.console import ConsoleError

from .screen import Screen
from .waiting import until

# On the device, the screen takes up to 7 s to refresh when cold. What follows a control may
# take a few refreshes.
SCREEN_TIMEOUT_S = 20
# The console's answer before the first frame is shown (src/maintenance/src/console.rs).
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
        """The screen once `condition(screen)` is true. If it never is, the error gives the
        last screen seen."""
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
