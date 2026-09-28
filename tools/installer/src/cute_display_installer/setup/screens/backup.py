import os

from .choice import ChoiceScreen
from .message import Tone


class BackupScreen(ChoiceScreen):
    question = 'Back up the clock first? Recommended: should anything go wrong, the backup puts it back as it is.'

    def on_mount(self):
        self.query_one('#body').update(
            f"Its whole memory, 16 MB, into a file in {os.getcwd()}; a few minutes, nothing on the clock "
            "changes. The file holds your Wi-Fi password: keep it to yourself.")
        self.show_buttons(('Back up', 'yes'), ('Skip', 'no'))

    def on_button_pressed(self, event):
        if event.button.id == 'yes':
            self.say('Backing up: a few minutes...')
            self.work(lambda: self.app.clock.back_up(os.getcwd()), self._backed_up, self._failed)
        else:
            self.app.after_backup()

    def _backed_up(self, path):
        self.say(f'Backed up into {path}. Keep it to yourself: it holds your Wi-Fi password.', Tone.DONE)
        self.show_buttons(('Next', 'next'))

    def _failed(self, error):
        self.say(f'Could not back up: {error}', Tone.PROBLEM)
        self.show_buttons(('Try again', 'yes'), ('Skip', 'no'))
