from .choice import ChoiceScreen
from .message import Tone


class DoneScreen(ChoiceScreen):
    question = 'Done'

    def __init__(self, summary):
        super().__init__()
        self.summary = summary

    def on_mount(self):
        self.say(self.summary, Tone.DONE)
        if self.app.running:
            self.query_one('#body').update("To go back to Habity's firmware: `cute-display boot habity`.")
        self.show_buttons(('Quit', 'quit'))

    def on_button_pressed(self):
        self.app.exit()
