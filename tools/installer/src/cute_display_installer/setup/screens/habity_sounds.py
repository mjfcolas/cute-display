from ...config.habity_sounds import RINGTONES, ZEN_SOUNDS
from .choice import ChoiceScreen


class HabitySoundsScreen(ChoiceScreen):
    """Whether to copy Habity's `sounds` for their app."""
    sounds = None
    without_them = ''

    def on_mount(self):
        names = self.app.answers.habity_sounds[self.sounds.app]
        titles = ', '.join(self.sounds.copy_name(name).rpartition('.')[0] for name in names)
        self.query_one('#body').update(f'{titles}: copied on the card into {self.sounds.directory}/, Habity keeping its own. '
                                       f'{self.without_them}')
        self.show_buttons(('Copy them', 'copy'), ('No', 'no'))

    def on_button_pressed(self, event):
        if event.button.id == 'copy':
            self.app.answers.copied.add(self.sounds.app)
        else:
            self.app.answers.copied.discard(self.sounds.app)
        self.app.advance(self)


class RingtonesScreen(HabitySoundsScreen):
    question = "Copy Habity's alarm ringtones for the alarm clock to choose from?"
    sounds = RINGTONES
    without_them = 'Without them, the alarm has its chime.'


class ZenSoundsScreen(HabitySoundsScreen):
    question = "Copy Habity's sleep sounds for Zen to choose from?"
    sounds = ZEN_SOUNDS
    without_them = 'Without them, Zen has its brown noise.'
