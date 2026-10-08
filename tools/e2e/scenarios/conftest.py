"""Each scenario on a simulator of its own: headless but with `--window`, on a copy of
cards/standard/, the web answered from web/, its time AT_THE_RECORDING unless it gives
one. With `--device`, the scenarios marked `device` on the device plugged in, running the
test image: its directory on the card laid out the same, the web and the time from stubs
on this computer."""
import shutil
import subprocess
import tempfile
from datetime import datetime, timezone
from pathlib import Path

import pytest

from cute_display_e2e.device import OWN_DIRECTORY, Device
from cute_display_e2e.display import Display
from cute_display_e2e.log import Log
from cute_display_e2e.simulator import Simulator
from cute_display_e2e.stub_time_server import FakeRunningTime, StubTimeServer
from cute_display_e2e.stub_web_server import StubWebServer, e2e_conf, local_address
from cute_display_link import card, usb
from cute_display_link import clock as rtc

REPOSITORY = Path(__file__).resolve().parents[3]
E2E = Path(__file__).resolve().parents[1]
STANDARD_CARD = E2E / 'cards' / 'standard' / 'cute-display'
# When web/ was recorded, so that the forecast is about the day the clock is on.
AT_THE_RECORDING = datetime(2026, 10, 1, 12, 12, tzinfo=timezone.utc)
# A clock minute in six seconds: time enough for a scenario to act within one, as a
# snooze said to the minute needs. The device runs at its own pace.
SPEED = 10


def pytest_addoption(parser):
    parser.addoption('--window', action='store_true', help="each simulator in its window, to watch what the scenario does")
    parser.addoption('--device', action='store_true', help="the scenarios marked device, on the device running the test image")
    parser.addoption('--stubs-address', help="where the device reaches this computer; else the address it leaves from")


def pytest_collection_modifyitems(config, items):
    if config.getoption('--device'):
        not_for_the_device = pytest.mark.skip(reason='not marked device')
        for item in items:
            if 'device' not in item.keywords:
                item.add_marker(not_for_the_device)


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
        shutil.copytree(STANDARD_CARD, self.card / 'cute-display')

    def put_before_start(self, path_in_own_directory, contents):
        on_card = self.card / 'cute-display' / path_in_own_directory
        on_card.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(contents, bytes):
            on_card.write_bytes(contents)
        else:
            on_card.write_text(contents)

    def logs(self):
        return [line for simulator in self.simulators for line in simulator.log.lines()]

    def start(self, speed=SPEED, at=AT_THE_RECORDING):
        if self.simulators:
            self.simulators[-1].close()
        simulator = Simulator(self.binary, self.card, self.console, time_s=int(at.timestamp()), speed=speed, web=E2E / 'web', window=self.window)
        self.simulators.append(simulator)
        return Display(simulator.link)

    def read_file(self, path_in_own_directory):
        return card.read_file(self.simulators[-1].link, f'cute-display/{path_in_own_directory}')

    def rtc_time(self):
        return rtc.read(self.simulators[-1].link)

    def check_and_close(self):
        logs = self.logs()
        print('\n'.join(logs))
        crashes = [crash for simulator in self.simulators for crash in simulator.log.crashes()]
        ended = self.simulators and self.simulators[-1].has_ended()
        for simulator in self.simulators:
            simulator.close()
        assert not crashes, '\n'.join(crashes)
        assert not ended, 'the simulator ended on its own'


class LocalStubs:
    def __init__(self, address):
        self.address = address
        self.web = StubWebServer(address)
        self.time = StubTimeServer(running_time=FakeRunningTime())

    def conf(self):
        return e2e_conf(self.address, self.web.port, self.time.port).encode()

    def close(self):
        self.web.close()
        self.time.close()


class DeviceClock:
    """The device restarts at each start; its directory is laid out anew at the first, and
    the files put before a start are written on the card then."""

    def __init__(self, device, stubs):
        self.device = device
        device.log = Log()
        self.stubs = stubs
        self.to_put = {path.relative_to(STANDARD_CARD).as_posix(): path.read_bytes() for path in STANDARD_CARD.rglob('*') if path.is_file()}
        self.to_put['e2e.conf'] = stubs.conf()
        self.started = False

    def put_before_start(self, path_in_own_directory, contents):
        self.to_put[path_in_own_directory] = contents if isinstance(contents, bytes) else contents.encode()

    def logs(self):
        self.device.keep_log()
        return self.device.log.lines()

    def start(self, speed=SPEED, at=AT_THE_RECORDING):
        del speed
        if not self.started:
            self.device.empty_own_directory_but_the_clocks_wifi()
            self.started = True
        self.device.put(self.to_put)
        self.to_put = {}
        self.stubs.time.time.set(at.timestamp())
        rtc.set_to(self.device.link, at)
        self.device.restart()
        return Display(self.device.link)

    def read_file(self, path_in_own_directory):
        return card.read_file(self.device.link, f'{OWN_DIRECTORY}/{path_in_own_directory}')

    def rtc_time(self):
        return rtc.read(self.device.link)

    def check_and_close(self):
        logs = self.logs()
        print('\n'.join(logs))
        crashes = self.device.log.crashes()
        assert not crashes, '\n'.join(crashes)


@pytest.fixture(scope='session')
def device(request):
    if not request.config.getoption('--device'):
        return None
    plugged = Device(usb.open_link())
    stubs = LocalStubs(request.config.getoption('--stubs-address') or local_address())
    request.addfinalizer(stubs.close)
    request.addfinalizer(plugged.close)
    return plugged, stubs


@pytest.fixture
def clock(request, device):
    if device:
        on_the_device = DeviceClock(*device)
        yield on_the_device
        on_the_device.check_and_close()
        return
    # A Unix socket's path is short: not under pytest's own temporary directory.
    with tempfile.TemporaryDirectory(prefix='cute-display-') as root:
        simulated = SimulatedClock(request.getfixturevalue('simulator_binary'), Path(root), request.config.getoption('--window'))
        yield simulated
        simulated.check_and_close()


@pytest.fixture
def display(clock):
    return clock.start()
