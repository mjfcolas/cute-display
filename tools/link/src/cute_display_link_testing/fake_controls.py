"""The buttons and the wheel: what they are sent, kept."""

HOLD_MS_AT_MOST = 10_000


class FakeControls:
    def __init__(self):
        self.sent = []

    def answer(self, verb, rest):
        if verb not in ('tap', 'hold', 'turn'):
            return None
        if verb == 'hold' and int(rest.split(' ', 1)[0]) > HOLD_MS_AT_MOST:
            return [f'error hold: {HOLD_MS_AT_MOST} ms at most']
        self.sent.append(f'{verb} {rest}')
        return ['ok']
