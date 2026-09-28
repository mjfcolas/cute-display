from .choice import ChoiceScreen
from .message import Tone


class SetupNowScreen(ChoiceScreen):
    question = 'Set Cute Display up now?'

    def on_mount(self):
        self.query_one('#body').update('Your Wi-Fi, your town, your time zone and the airports on the radar. '
                                       'What the clock already holds is filled in.')
        self.show_buttons(('Set it up', 'yes'), ('Not now', 'no'))

    def on_button_pressed(self, event):
        if event.button.id == 'yes':
            self.say('Reading what the clock holds...')
            self.work(self.app.clock.read_card, self.app.after_card_read, self._failed)
        else:
            self.app.finish('Nothing more to do. `cute-display setup` again sets the clock up whenever you like.')

    def _failed(self, error):
        self.say(f'Could not read the clock: {error}', Tone.PROBLEM)
        self.show_buttons(('Try again', 'yes'), ('Not now', 'no'))
