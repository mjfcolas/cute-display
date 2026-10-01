"""Each scenario on a simulator of its own: headless but with `--window`, on a copy of
cards/standard/, the web answered from web/, its time AT_THE_RECORDING unless it gives
one."""
import shutil
import subprocess
import tempfile
from datetime import datetime, timezone
from pathlib import Path

import pytest

from cute_display_e2e.display import Display
from cute_display_e2e.simulator import Simulator

REPOSITORY = Path(__file__).resolve().parents[3]
E2E = Path(__file__).resolve().parents[1]
# When web/ was recorded, so that the forecast is about the day the clock is on.
AT_THE_RECORDING = datetime(2026, 10, 1, 12, 12, tzinfo=timezone.utc)
# A clock minute in six seconds: time enough for a scenario to act within one, as a
# snooze said to the minute needs.
SPEED = 10


def pytest_addoption(parser):
    parser.addoption('--window', action='store_true', help="each simulator in its window, to watch what the scenario does")


@pytest.fixture(scope='session')
def simulator_binary():
    subprocess.run(['cargo', 'build', '--quiet', '-p', 'simulator'], cwd=REPOSITORY, check=True)
    return REPOSITORY / 'target' / 'debug' / 'simulator'


class SimulatedClock:

    def __init__(self, binary, root, window):
        self.binary = binary
        self.window = window
        self.card = root / 'card'
        self.console = root / 'console.sock'
        self.simulators = []
        shutil.copytree(E2E / 'cards' / 'standard', self.card)

    def put_before_start(self, path, text):
        (self.card / path).parent.mkdir(parents=True, exist_ok=True)
        (self.card / path).write_text(text)

    def start(self, speed=SPEED, at=AT_THE_RECORDING):
        if self.simulators:
            self.simulators[-1].close()
        simulator = Simulator(self.binary, self.card, self.console, time_s=int(at.timestamp()), speed=speed, web=E2E / 'web', window=self.window)
        self.simulators.append(simulator)
        return Display(simulator.link)

    def check_and_close(self):
        logs = [line for simulator in self.simulators for line in simulator.log.lines()]
        print('\n'.join(logs))
        crashes = [crash for simulator in self.simulators for crash in simulator.log.crashes()]
        ended = self.simulators and self.simulators[-1].has_ended()
        for simulator in self.simulators:
            simulator.close()
        assert not crashes, '\n'.join(crashes)
        assert not ended, 'the simulator ended on its own'


@pytest.fixture
def clock(request, simulator_binary):
    # A Unix socket's path is short: not under pytest's own temporary directory.
    with tempfile.TemporaryDirectory(prefix='cute-display-') as root:
        simulated = SimulatedClock(simulator_binary, Path(root), request.config.getoption('--window'))
        yield simulated
        simulated.check_and_close()


@pytest.fixture
def display(clock):
    return clock.start()
