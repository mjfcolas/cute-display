"""The SD card, on files in memory."""
import base64
import zlib

RANGE_BYTES = 100


class FakeCard:
    def __init__(self, files, damaged_ranges=0):
        self.files = dict(files)
        self.damaged_ranges = damaged_ranges
        self.receiving = None

    def _children(self, directory):
        prefix = f'{directory}/' if directory else ''
        children = {}
        for path, contents in self.files.items():
            if path.startswith(prefix):
                name, _, below = path[len(prefix):].partition('/')
                children[name] = None if below else len(contents)
        return children

    def answer(self, verb, rest):
        if verb == 'ls':
            listed = [f'entry d 0 {name}' if size is None else f'entry f {size} {name}'
                      for name, size in self._children(rest).items()]
            return [*listed, 'ok']
        if verb == 'get':
            offset, path = rest.split(' ', 1)
            chunk = self.files[path][int(offset):int(offset) + RANGE_BYTES]
            sent = chunk
            if self.damaged_ranges:
                self.damaged_ranges -= 1
                sent = chunk[:-1]
            return [f'data {base64.b64encode(sent).decode()}', f'ok {len(chunk)} {zlib.crc32(chunk)}']
        if verb == 'put':
            size, crc, path = rest.split(' ', 2)
            if not path.startswith('cute-display/'):
                return ['error only cute-display/ can be written']
            self.receiving = (path, int(size), int(crc), bytearray())
            return ['ready']
        if verb == 'data':
            self.receiving[3].extend(base64.b64decode(rest))
            return ['ack']
        if verb == 'end':
            path, size, crc, contents = self.receiving
            assert len(contents) == size and zlib.crc32(contents) == crc
            self.files[path] = bytes(contents)
            return ['ok']
        if verb == 'cp':
            source, destination = rest.split('\t')
            if not destination.startswith('cute-display/'):
                return ['error only cute-display/ may be written']
            self.files[destination] = self.files[source]
            return ['ok']
        if verb == 'rm':
            del self.files[rest]
            return ['ok']
        return None
