from textual.containers import Vertical
from textual.screen import Screen
from textual.widgets import Button, Footer, Header, Static

from ..card import Unreachable
from .message import Message, Tone


class SummaryScreen(Screen):
    written = False

    def compose(self):
        answers = self.app.answers
        labels = ', '.join(answers.labels) or 'none'
        yield Header()
        with Vertical(classes='step'):
            yield Static('What goes onto the device:', classes='question')
            yield Static('\n'.join([
                f'Wi-Fi       {answers.wifi.ssid}, {"with a password" if answers.wifi.password else "open"}',
                f'Place       {answers.place.name} ({answers.place.latitude:.4f}, {answers.place.longitude:.4f})',
                f'Time zone   {answers.time_zone}',
                f'Radar       {len(self.app.airports_found)} airports, codes on {labels}',
            ]))
            yield Message()
            yield Button('Write to the device', variant='primary')
        yield Footer()

    def on_button_pressed(self, event):
        if self.written:
            self.app.exit()
            return
        event.button.disabled = True
        self.query_one(Message).say('Writing...')
        self.run_worker(self._write, thread=True, exclusive=True)

    def _write(self):
        try:
            self.app.write(self.app.answers.files(self.app.airports_found))
        except Unreachable as error:
            self.app.call_from_thread(self._failed, error)
            return
        self.app.call_from_thread(self._written)

    def _failed(self, error):
        self.query_one(Message).say(f'Could not write: {error}', Tone.PROBLEM)
        self.query_one(Button).disabled = False

    def _written(self):
        self.query_one(Message).say(
            'Written. The clock takes the time zone within a minute, the weather at its next update (a long '
            'press in its app updates now), the radar its airports at its next start or when its place changes.',
            Tone.DONE)
        self.written = True
        button = self.query_one(Button)
        button.label, button.disabled = 'Quit', False
