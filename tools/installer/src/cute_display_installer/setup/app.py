"""The setup assistant: a screen per question, then what it writes onto the card."""
from textual.app import App

from .screens.airports import AirportsScreen
from .screens.place import PlaceScreen
from .screens.summary import SummaryScreen
from .screens.time_zone import TimeZoneScreen
from .screens.wifi import WifiScreen

STEPS = [WifiScreen, PlaceScreen, TimeZoneScreen, AirportsScreen, SummaryScreen]


class SetupApp(App):
    TITLE = 'Cute Display setup'
    CSS_PATH = 'setup.tcss'
    BINDINGS = [('escape', 'back', 'Back'), ('ctrl+q', 'quit', 'Quit')]

    def __init__(self, answers, search, airports_around, write):
        """`search` finds places by name, `airports_around` a place's airports, and
        `write` puts files onto the card, raising `card.Unreachable`; each may take seconds."""
        super().__init__()
        self.answers = answers
        self.search = search
        self.airports_around = airports_around
        self.write = write
        self.airports_found = []

    def on_mount(self):
        self.push_screen(STEPS[0]())

    def advance(self, screen):
        self.push_screen(STEPS[STEPS.index(type(screen)) + 1]())

    def action_back(self):
        if isinstance(self.screen, tuple(STEPS[1:])):
            self.pop_screen()
