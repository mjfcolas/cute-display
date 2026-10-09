"""The line of a screen that says what is happening, in a colour that matches its tone."""
from enum import Enum

from textual.widgets import Static


class Tone(Enum):
    NEWS = 'news'
    PROBLEM = 'problem'
    DONE = 'done'


class Message(Static):
    def say(self, text, tone=Tone.NEWS):
        self.update(text)
        for each in Tone:
            self.set_class(each is tone, each.value)
