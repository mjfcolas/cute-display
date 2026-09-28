from textual.containers import Vertical
from textual.screen import Screen
from textual.widgets import Button, Footer, Header, SelectionList, Static

from ...config import apps


class AppsScreen(Screen):
    def compose(self):
        chosen = self.app.answers.apps
        self.order = chosen + [name for name in apps.TITLES if name not in chosen]
        yield Header()
        with Vertical(classes='step'):
            yield Static('The apps on the clock; the first one checked is on screen when it starts.',
                         classes='question')
            yield SelectionList(*[(apps.TITLES[name], name, name in chosen) for name in self.order], id='apps')
            yield Button('Next', variant='primary')
        yield Footer()

    def on_button_pressed(self):
        selected = self.query_one('#apps', SelectionList).selected
        self.app.answers.apps = [name for name in self.order if name in selected]
        self.app.advance(self)
