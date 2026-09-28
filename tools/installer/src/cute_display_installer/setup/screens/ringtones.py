from ...config.ringtones import DIRECTORY
from .choice import ChoiceScreen


class RingtonesScreen(ChoiceScreen):
    question = "Copy Habity's alarm ringtones for the alarm clock to choose from?"

    def on_mount(self):
        names = self.app.answers.habity_ringtones
        titles = ', '.join(name.rpartition('.')[0] for name in names)
        self.query_one('#body').update(f'{titles}: copied on the card into {DIRECTORY}/, Habity keeping its own. '
                                       'Without them, the alarm has its chime.')
        self.show_buttons(('Copy them', 'copy'), ('No', 'no'))

    def on_button_pressed(self, event):
        self.app.answers.copy_ringtones = event.button.id == 'copy'
        self.app.advance(self)
