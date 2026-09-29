"""The device's side of the console, on files in memory, with its log between the
replies: for tests of what talks to it. The controls it is sent, it keeps."""
import base64
import zlib

RANGE_BYTES = 100
HOLD_MS_AT_MOST = 10_000


class Card:
    def __init__(self, files, damaged_ranges=0):
        self.files = dict(files)
        self.damaged_ranges = damaged_ranges
        self.replies = []
        self.receiving = None
        self.controls = []

    def write(self, data):
        prefix, request_id, words = data.decode().rstrip('\n').split(' ', 2)
        assert prefix == '@@'
        self.replies.append(b'I (1234) app: a log line\n')
        self._answer(request_id, *words.split(' ', 1))

    def flush(self):
        pass

    def readline(self):
        return self.replies.pop(0) if self.replies else b''

    def _reply(self, request_id, words):
        self.replies.append(f'@@ {request_id} {words}\n'.encode())

    def _children(self, directory):
        prefix = f'{directory}/' if directory else ''
        children = {}
        for path, contents in self.files.items():
            if path.startswith(prefix):
                name, _, below = path[len(prefix):].partition('/')
                children[name] = None if below else len(contents)
        return children

    def _answer(self, request_id, verb, rest=''):
        if verb == 'ls':
            for name, size in self._children(rest).items():
                self._reply(request_id, f'entry d 0 {name}' if size is None else f'entry f {size} {name}')
            self._reply(request_id, 'ok')
        elif verb == 'get':
            offset, path = rest.split(' ', 1)
            chunk = self.files[path][int(offset):int(offset) + RANGE_BYTES]
            sent = chunk
            if self.damaged_ranges:
                self.damaged_ranges -= 1
                sent = chunk[:-1]
            self._reply(request_id, f'data {base64.b64encode(sent).decode()}')
            self._reply(request_id, f'ok {len(chunk)} {zlib.crc32(chunk)}')
        elif verb == 'put':
            size, crc, path = rest.split(' ', 2)
            if not path.startswith('cute-display/'):
                self._reply(request_id, 'error only cute-display/ can be written')
                return
            self.receiving = (path, int(size), int(crc), bytearray())
            self._reply(request_id, 'ready')
        elif verb == 'data':
            self.receiving[3].extend(base64.b64decode(rest))
            self._reply(request_id, 'ack')
        elif verb == 'end':
            path, size, crc, contents = self.receiving
            assert len(contents) == size and zlib.crc32(contents) == crc
            self.files[path] = bytes(contents)
            self._reply(request_id, 'ok')
        elif verb == 'cp':
            source, destination = rest.split('\t')
            if not destination.startswith('cute-display/'):
                self._reply(request_id, 'error only cute-display/ may be written')
                return
            self.files[destination] = self.files[source]
            self._reply(request_id, 'ok')
        elif verb == 'rm':
            del self.files[rest]
            self._reply(request_id, 'ok')
        elif verb == 'hold' and int(rest.split(' ', 1)[0]) > HOLD_MS_AT_MOST:
            self._reply(request_id, f'error hold: {HOLD_MS_AT_MOST} ms at most')
        elif verb in ('tap', 'hold', 'turn'):
            self.controls.append(f'{verb} {rest}')
            self._reply(request_id, 'ok')
