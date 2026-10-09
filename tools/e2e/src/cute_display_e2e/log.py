"""What the app image logged, as it arrives."""
import re
import threading

# What the app image logs only when something went badly wrong.
CRASH = re.compile(r'panicked|Guru Meditation|stack overflow|abort\(\)|Rebooting')
# ESP-IDF colours its lines.
ANSI_ESCAPE = re.compile(r'\x1b\[[0-9;]*m')


class Log:
    def __init__(self):
        self._lines = []
        self._lock = threading.Lock()

    def add(self, line):
        with self._lock:
            self._lines.append(ANSI_ESCAPE.sub('', line).rstrip())

    def lines(self):
        with self._lock:
            return list(self._lines)

    def crashes(self):
        return [line for line in self.lines() if CRASH.search(line)]
