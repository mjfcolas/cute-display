from urllib.error import URLError

from textual.containers import Vertical
from textual.screen import Screen
from textual.widgets import Button, Footer, Header, Input, OptionList, Static

from ...config import time_zone
from .message import Message, Tone


def _described(place):
    return f'{place.name} ({place.latitude:.4f}, {place.longitude:.4f})' if place else 'none yet'


class PlaceScreen(Screen):
    """Where the weather and the radar are about; choosing it also suggests its time zone."""

    def compose(self):
        yield Header()
        with Vertical(classes='step'):
            yield Static("The place for the weather and the radar: search a town, then choose it.",
                         classes='question')
            yield Static(f'Now: {_described(self.app.answers.place)}', id='now')
            yield Input(placeholder='A town, e.g. Lyon', id='search')
            yield OptionList(id='found')
            yield Message()
            yield Button('Next', variant='primary')
        yield Footer()

    def on_input_submitted(self, event):
        self.query_one(Message).say('Searching...')
        name = event.value.strip()
        self.run_worker(lambda: self._search(name), thread=True, exclusive=True)

    def _search(self, name):
        try:
            found = self.app.search(name)
        except (URLError, OSError, ValueError) as error:
            self.app.call_from_thread(self.query_one(Message).say, f'Could not search: {error}', Tone.PROBLEM)
            return
        self.app.call_from_thread(self._list_places, found)

    def _list_places(self, found):
        self.found = found
        options = self.query_one('#found', OptionList)
        options.clear_options()
        options.add_options([str(f) for f in found])
        self.query_one(Message).say('' if found else 'Nothing found by that name.', Tone.NEWS if found else Tone.PROBLEM)
        if found:
            options.highlighted = 0
            options.focus()

    def on_option_list_option_selected(self, event):
        chosen = self.found[event.option_index]
        self.app.answers.place = chosen.place
        if time_zone.is_known(chosen.time_zone):
            self.app.answers.time_zone = chosen.time_zone
        self.app.advance(self)

    def on_button_pressed(self):
        if self.app.answers.place is None:
            self.query_one(Message).say('Search a town and choose it first.', Tone.PROBLEM)
            return
        self.app.advance(self)
