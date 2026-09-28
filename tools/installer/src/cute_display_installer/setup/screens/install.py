from ...flash.layout import APP_HEADER_SIZE, app_image
from ..offer import Offering, offer
from .choice import ChoiceScreen
from .message import Tone


class InstallScreen(ChoiceScreen):
    question = 'Cute Display on the clock'

    def __init__(self):
        super().__init__()
        self.contents = None
        self.offered = None

    def on_mount(self):
        self.say('Looking for the latest release...')
        self.work(self.app.clock.latest, self._found, self._failed)

    def _found(self, contents):
        self.contents = contents
        self.offered = offer(self.app.unit, app_image(contents[:APP_HEADER_SIZE]))
        offering, latest, installed = self.offered.offering, self.offered.latest, self.offered.installed
        if offering is Offering.INSTALL:
            erasing = self.offered.placement.erases
            self.say(f'Install {latest} {self.offered.placement}?', Tone.PROBLEM if erasing else Tone.NEWS)
            self.show_buttons(('Install', 'install'), ('Not now', 'done'))
        elif offering is Offering.UPDATE:
            self.say(f'{installed} is installed; update it to {latest}?')
            self.show_buttons(('Update', 'install'), ('Not now', 'done'))
        elif offering is Offering.START:
            self.say(f"{installed} is installed, but the clock runs Habity's firmware: start Cute Display?")
            self.show_buttons(('Start it', 'start'), ('Not now', 'done'))
        else:
            self.say(f'{installed} is installed and up to date.', Tone.DONE)
            self.show_buttons(('Next', 'done'))

    def _failed(self, error):
        self.say(f'Could not do it: {error}', Tone.PROBLEM)
        self.show_buttons(('Try again', 'again'), ('Skip', 'done'))

    def _cute_display_runs(self, _):
        self.app.running = True
        self.say('Cute Display runs on the clock.', Tone.DONE)
        self.show_buttons(('Next', 'done'))

    def on_button_pressed(self, event):
        clock = self.app.clock
        if event.button.id == 'install':
            self.say('Installing, then waiting for the clock to start it: a minute or two...')
            self.work(lambda: (clock.install(self.contents), clock.wait_for_cute_display()), self._cute_display_runs,
                      self._failed)
        elif event.button.id == 'start':
            self.say('Starting Cute Display...')
            self.work(lambda: (clock.start(self.offered.slot), clock.wait_for_cute_display()), self._cute_display_runs,
                      self._failed)
        elif event.button.id == 'again':
            self.on_mount()
        else:
            self.app.after_install()
