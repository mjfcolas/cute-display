"""Habity's sounds that one of our apps can play: the MP3 and WAV files in Habity's
directory, as src/libs/sound_files/ lists them. The setup offers to copy them into the app's
directory.
"""
from dataclasses import dataclass

PLAYABLE = ('.mp3', '.wav')


@dataclass(frozen=True)
class HabitySounds:
    app: str
    title: str
    habity_directory: str
    directory: str
    # The end of Habity's file names, which the copies' names do not have.
    habity_ending: str = ''

    def copy_name(self, name):
        stem, dot, extension = name.rpartition('.')
        return stem.removesuffix(self.habity_ending) + dot + extension

    def copies(self, names):
        """Each of Habity's sounds by name, and where its copy goes."""
        return {f'{self.habity_directory}/{name}': f'{self.directory}/{self.copy_name(name)}' for name in names}


def is_playable(name):
    return name.lower().endswith(PLAYABLE)


RINGTONES = HabitySounds('alarm', 'Ringtones', 'sounds/alarm', 'cute-display/apps/alarm/ringtones')
ZEN_SOUNDS = HabitySounds('zen', 'Zen sounds', 'sounds/sleep', 'cute-display/apps/zen/sounds', habity_ending='_loop')
EVERY = [RINGTONES, ZEN_SOUNDS]
