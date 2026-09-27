from urllib.error import URLError

from textual.containers import Vertical
from textual.screen import Screen
from textual.widgets import Button, Footer, Header, SelectionList, Static

from ...config.airports import RADIUS_KM
from .message import Message, Tone


class AirportsScreen(Screen):
    def compose(self):
        yield Header()
        with Vertical(classes='step'):
            yield Static('Airports on the radar: the checked ones carry their code, the others are dots.',
                         classes='question')
            yield Message(f'Looking for airports around {self.app.answers.place.name}...')
            yield SelectionList(id='airports')
            yield Button('Next', variant='primary', disabled=True)
        yield Footer()

    def on_mount(self):
        self.run_worker(self._find_airports, thread=True, exclusive=True)

    def _find_airports(self):
        try:
            around = self.app.airports_around(self.app.answers.place)
            news = f'{len(around)} airports within {RADIUS_KM} km.', Tone.NEWS
        except (URLError, OSError) as error:
            around = []
            news = f'Could not get the airports ({error}): the radar will show none.', Tone.PROBLEM
        self.app.call_from_thread(self._list_airports, around, news)

    def _list_airports(self, around, news):
        self.app.airports_found = around
        labelled = set(self.app.answers.labels)
        known = labelled & {airport.code for airport in around}
        self.query_one('#airports', SelectionList).add_options([
            (f'{airport.code:<8} {airport.name} ({airport.distance_km:.0f} km)', airport.code,
             airport.code in known if known else airport.is_large)
            for airport in around])
        self.query_one(Message).say(*news)
        self.query_one(Button).disabled = False

    def on_button_pressed(self):
        self.app.answers.labels = list(self.query_one('#airports', SelectionList).selected)
        self.app.advance(self)
