"""The setup: back up the clock, see what it holds, install or update Cute Display, then
answer a screen per question and write the answers onto the card."""
from textual.app import App

from .screens.airports import AirportsScreen
from .screens.backup import BackupScreen
from .screens.done import DoneScreen
from .screens.install import InstallScreen
from .screens.place import PlaceScreen
from .screens.setup_now import SetupNowScreen
from .screens.state import StateScreen
from .screens.summary import SummaryScreen
from .screens.time_zone import TimeZoneScreen
from .screens.wifi import WifiScreen

QUESTIONS = [WifiScreen, PlaceScreen, TimeZoneScreen, AirportsScreen, SummaryScreen]


class SetupApp(App):
    TITLE = 'Cute Display setup'
    CSS_PATH = 'setup.tcss'
    BINDINGS = [('escape', 'back', 'Back'), ('ctrl+q', 'quit', 'Quit')]

    def __init__(self, clock, search, airports_around):
        """`clock` is the device (`clock.UsbClock`), `search` finds places by name and
        `airports_around` a place's airports; each may take seconds."""
        super().__init__()
        self.clock = clock
        self.search = search
        self.airports_around = airports_around
        self.unit = None
        self.running = False
        self.answers = None
        self.airports_found = []

    def on_mount(self):
        self.push_screen(BackupScreen())

    def after_backup(self):
        self.push_screen(StateScreen())

    def after_state(self):
        ours = self.unit.ours()
        self.running = ours is not None and self.unit.booting == ours
        self.push_screen(InstallScreen())

    def after_install(self):
        if self.running:
            self.push_screen(SetupNowScreen())
        else:
            self.finish('Cute Display does not run on the clock, so it was not set up.')

    def after_card_read(self, answers):
        self.answers = answers
        self.push_screen(QUESTIONS[0]())

    def advance(self, screen):
        self.push_screen(QUESTIONS[QUESTIONS.index(type(screen)) + 1]())

    def finish(self, summary):
        self.push_screen(DoneScreen(summary))

    def action_back(self):
        if isinstance(self.screen, tuple(QUESTIONS[1:])):
            self.pop_screen()
