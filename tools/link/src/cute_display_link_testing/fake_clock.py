"""The RTC, stopped at the time a test gives, or it was set to."""


class FakeClock:
    def __init__(self, unix_seconds):
        self.unix_seconds = unix_seconds

    def answer(self, verb, rest):
        if verb != 'clock':
            return None
        if rest.startswith('set '):
            self.unix_seconds = int(rest.split(' ', 1)[1])
            return ['ok']
        return [f'ok {self.unix_seconds}']
