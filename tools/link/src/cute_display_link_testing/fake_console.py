"""The device's side of its console, for tests of what talks to it: each request goes to
the part that knows its verb, and the log comes between the replies."""


class FakeConsole:
    def __init__(self, *parts):
        self.parts = parts
        self.replies = []

    def write(self, data):
        prefix, request_id, words = data.decode().rstrip('\n').split(' ', 2)
        assert prefix == '@@'
        verb, _, rest = words.partition(' ')
        answers = (part.answer(verb, rest) for part in self.parts)
        answer = next((answer for answer in answers if answer is not None), None)
        assert answer is not None, f'no part of this fake answers {verb}'
        self.replies.append(b'I (1234) app: a log line\n')
        self.replies += [f'@@ {request_id} {words}\n'.encode() for words in answer]

    def flush(self):
        pass

    def readline(self):
        return self.replies.pop(0) if self.replies else b''
