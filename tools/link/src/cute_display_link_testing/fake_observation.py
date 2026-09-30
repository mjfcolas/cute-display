"""The lights and the speaker, as a test sets them."""


class FakeObservation:
    def __init__(self, lights=(0, 0), playing=False):
        self.lights = lights
        self.playing = playing

    def answer(self, verb, rest):
        if verb == 'lights':
            return ['ok {} {}'.format(*self.lights)]
        if verb == 'sound':
            return ['ok playing' if self.playing else 'ok silent']
        return None
