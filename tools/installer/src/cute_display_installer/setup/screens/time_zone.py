from textual.containers import Vertical
from textual.screen import Screen
from textual.widgets import Button, Footer, Header, Input, OptionList, Static

from ...config import time_zone
from .message import Message, Tone


class TimeZoneScreen(Screen):
    def compose(self):
        self.zones = time_zone.names()
        chosen = self.app.answers.time_zone or time_zone.of_this_computer()
        yield Header()
        with Vertical(classes='step'):
            yield Static("The clock's time zone: the place's own, unless another is chosen.", classes='question')
            yield Input(chosen, placeholder='Type to filter, e.g. Europe/', id='filter')
            yield OptionList(id='zones')
            yield Message()
            yield Button('Next', variant='primary')
        yield Footer()

    def on_mount(self):
        self._list(self.query_one('#filter').value)

    def on_input_changed(self, event):
        self._list(event.value)

    def _list(self, text):
        self.shown = [zone for zone in self.zones if text.strip().lower() in zone.lower()]
        options = self.query_one('#zones', OptionList)
        options.clear_options()
        options.add_options(self.shown)

    def on_option_list_option_selected(self, event):
        self._choose(self.shown[event.option_index])

    def on_input_submitted(self):
        self._confirm()

    def on_button_pressed(self):
        self._confirm()

    def _confirm(self):
        text = self.query_one('#filter').value.strip()
        if text in self.zones:
            self._choose(text)
        elif len(self.shown) == 1:
            self._choose(self.shown[0])
        else:
            self.query_one(Message).say('Choose a zone in the list.', Tone.PROBLEM)

    def _choose(self, zone):
        self.app.answers.time_zone = zone
        self.app.advance(self)
