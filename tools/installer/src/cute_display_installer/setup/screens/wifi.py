from textual.containers import Vertical
from textual.screen import Screen
from textual.widgets import Button, Footer, Header, Input, Static

from ...config.wifi import Wifi
from ..answers import wifi_problem
from .message import Message, Tone


class WifiScreen(Screen):
    def compose(self):
        known = self.app.answers.wifi
        yield Header()
        with Vertical(classes='step'):
            yield Static('The Wi-Fi network the device joins, for the time, the weather and the radar.',
                         classes='question')
            yield Input(known.ssid if known else '', placeholder='Network name', id='ssid')
            yield Input(known.password if known else '', placeholder='Password, none for an open network',
                        password=True, id='password')
            yield Message()
            yield Button('Next', variant='primary')
        yield Footer()

    def on_input_submitted(self, event):
        if event.input.id == 'ssid':
            self.query_one('#password').focus()
        else:
            self.answer()

    def on_button_pressed(self):
        self.answer()

    def answer(self):
        ssid, password = self.query_one('#ssid').value, self.query_one('#password').value
        problem = wifi_problem(ssid, password)
        if problem:
            self.query_one(Message).say(problem, Tone.PROBLEM)
            return
        self.app.answers.wifi = Wifi(ssid, password)
        self.app.advance(self)
