"""The app image on this computer, on a card of its own and at a time given, its console
on a Unix socket (docs/simulator/README.md)."""
import subprocess
import threading
import time
from pathlib import Path

from cute_display_link.simulator import NoSimulator, SimulatorLink

from .log import Log

STARTING_TIMEOUT_S = 20
ENDING_TIMEOUT_S = 5


class Simulator:
    def __init__(self, binary, card, console, *, time_s, speed, web=None, window=False):
        arguments = [str(binary), str(card), '--console', str(console), '--time', str(time_s), '--speed', str(speed)]
        arguments += ['--web', str(web)] if web else ['--offline']
        if not window:
            arguments.append('--headless')
        self.log = Log()
        self.process = subprocess.Popen(arguments, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True, errors='replace')
        self._log_reader = threading.Thread(target=self._read_log, daemon=True)
        self._log_reader.start()
        self.link = self._connect(Path(console))

    def _read_log(self):
        for line in self.process.stderr:
            self.log.add(line)

    def _connect(self, console):
        deadline = time.monotonic() + STARTING_TIMEOUT_S
        while True:
            if self.process.poll() is not None:
                self._log_reader.join(timeout=1)
                raise RuntimeError('the simulator ended at its start:\n' + '\n'.join(self.log.lines()))
            try:
                return SimulatorLink(console)
            except NoSimulator:
                if time.monotonic() > deadline:
                    self.close()
                    raise RuntimeError('the simulator did not open its console:\n' + '\n'.join(self.log.lines())) from None
                time.sleep(0.05)

    def has_ended(self):
        return self.process.poll() is not None

    def close(self):
        if getattr(self, 'link', None):
            self.link.close()
        self.process.terminate()
        try:
            self.process.wait(timeout=ENDING_TIMEOUT_S)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
