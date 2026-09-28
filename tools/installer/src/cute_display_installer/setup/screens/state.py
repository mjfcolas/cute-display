from .choice import ChoiceScreen
from .message import Tone

ISSUES = 'https://github.com/mjfcolas/cute-display/issues'


class StateScreen(ChoiceScreen):
    question = 'What the clock holds'

    def on_mount(self):
        self.say('Reading the clock...')
        self.work(self.app.clock.read, self._read, self._failed)

    def _read(self, unit):
        self.app.unit = unit
        self.query_one('#body').update('\n'.join(unit.slot_lines()))
        refusals = unit.install_refusals()
        if refusals and unit.ours() is None:
            self.say('\n'.join(f'Cannot install: {refusal}' for refusal in refusals)
                     + f'\nNothing was written; if unsure, open an issue: {ISSUES}', Tone.PROBLEM)
            self.show_buttons(('Quit', 'quit'))
        elif unit.ours() is not None:
            self.say(f'Cute Display is in {unit.ours()}.')
            self.show_buttons(('Next', 'next'))
        else:
            self.say(f'Cute Display can go {unit.placement()}.')
            self.show_buttons(('Next', 'next'))

    def _failed(self, error):
        self.say(f'Could not read the clock: {error}', Tone.PROBLEM)
        self.show_buttons(('Try again', 'again'), ('Quit', 'quit'))

    def on_button_pressed(self, event):
        if event.button.id == 'next':
            self.app.after_state()
        elif event.button.id == 'again':
            self.on_mount()
        else:
            self.app.exit()
