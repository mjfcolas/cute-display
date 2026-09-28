from textual.containers import Vertical
from textual.screen import Screen
from textual.widgets import Button, Footer, Header, Static

from ...config import apps
from ..clock import Failed
from .message import Message, Tone


class SummaryScreen(Screen):
    def compose(self):
        answers = self.app.answers
        labels = ', '.join(answers.labels) or 'none'
        chosen = ', '.join(apps.TITLES[name] for name in answers.apps) or 'none'
        radar = [f'Radar       {len(self.app.airports_found)} airports, codes on {labels}'] if 'radar' in answers.apps else []
        copies = answers.copies()
        ringtones = [f"Ringtones   Habity's {len(copies)} copied for the alarm"] if copies else []
        yield Header()
        with Vertical(classes='step'):
            yield Static('What goes onto the device:', classes='question')
            yield Static('\n'.join([
                f'Wi-Fi       {answers.wifi.ssid}, {"with a password" if answers.wifi.password else "open"}',
                f'Place       {answers.place.name} ({answers.place.latitude:.4f}, {answers.place.longitude:.4f})',
                f'Time zone   {answers.time_zone}',
                f'Apps        {chosen}',
                *ringtones,
                *radar,
            ]))
            yield Message()
            yield Button('Write to the device', variant='primary')
        yield Footer()

    def on_button_pressed(self, event):
        event.button.disabled = True
        copying = ', and copying the ringtones: a minute' if self.app.answers.copies() else ''
        self.query_one(Message).say(f'Writing{copying}...')
        self.run_worker(self._write, thread=True, exclusive=True)

    def _write(self):
        try:
            self.app.clock.write_card(self.app.answers.files(self.app.airports_found), self.app.answers.copies())
        except Failed as error:
            self.app.call_from_thread(self._failed, error)
            return
        try:
            self.app.clock.restart()
        except Failed as error:
            self.app.call_from_thread(self.app.finish, f'Written, but the clock did not restart ({error}): '
                                                        '`cute-display check` restarts it, to take it all.')
            return
        self.app.call_from_thread(self._written)

    def _failed(self, error):
        self.query_one(Message).say(f'Could not write: {error}', Tone.PROBLEM)
        self.query_one(Button).disabled = False

    def _written(self):
        self.app.finish('Written, and the clock restarted to take it all.')
