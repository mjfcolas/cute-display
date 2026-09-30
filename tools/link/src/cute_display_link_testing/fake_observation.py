"""The glass, what it says, the lights and the speaker, as a test sets them."""
import base64

DATA_CHUNK_BYTES = 240


class FakeObservation:
    def __init__(self, ink=None, times_shown=0, lines=None, lights=(0, 0), playing=False):
        self.ink = ink
        self.times_shown = times_shown
        self.lines = lines
        self.lights = lights
        self.playing = playing

    def answer(self, verb, rest):
        if verb == 'screen' and self.ink is None:
            return ['error nothing shown yet']
        if verb == 'screen':
            chunks = [self.ink[at:at + DATA_CHUNK_BYTES] for at in range(0, len(self.ink), DATA_CHUNK_BYTES)]
            return [f'data {base64.b64encode(chunk).decode()}' for chunk in chunks] + [f'ok {self.times_shown}']
        if verb == 'describe' and self.lines is None:
            return ['error nothing said yet']
        if verb == 'describe':
            return [f'text {line}' for line in self.lines] + [f'ok {self.times_shown}']
        if verb == 'lights':
            return ['ok {} {}'.format(*self.lights)]
        if verb == 'sound':
            return ['ok playing' if self.playing else 'ok silent']
        return None
