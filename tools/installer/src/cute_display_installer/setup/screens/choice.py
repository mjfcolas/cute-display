"""A step of the setup that is a question and its buttons, not a form."""
from textual.containers import Horizontal, Vertical
from textual.screen import Screen
from textual.widgets import Button, Footer, Header, Static

from ..clock import Failed
from .message import Message, Tone


class ChoiceScreen(Screen):
    question = ''

    def compose(self):
        yield Header()
        with Vertical(classes='step'):
            yield Static(self.question, classes='question')
            yield Static(id='body')
            yield Message()
            yield Horizontal(id='buttons')
        yield Footer()

    def show_buttons(self, *buttons):
        """Buttons as (label, id), the first one the likely answer."""
        row = self.query_one('#buttons', Horizontal)
        row.remove_children()
        row.mount_all([Button(label, id=button_id, variant='primary' if i == 0 else 'default')
                       for i, (label, button_id) in enumerate(buttons)])
        if buttons:
            row.children[0].focus()

    def say(self, text, tone=Tone.NEWS):
        self.query_one(Message).say(text, tone)

    def work(self, call, then, failed):
        """`call()` away from the screen, then `then(result)` or `failed(Failed)` on it."""
        def run():
            try:
                result = call()
            except Failed as error:
                self.app.call_from_thread(failed, error)
                return
            self.app.call_from_thread(then, result)
        self.show_buttons()
        self.run_worker(run, thread=True, exclusive=True)
