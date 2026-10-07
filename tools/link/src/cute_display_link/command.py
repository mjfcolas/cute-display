"""What the link's commands share: the link they open, the device's or a simulator's, and
what they say when it fails."""
import sys

from serial import SerialException

from . import usb
from .console import ConsoleError
from .simulator import NoSimulator, SimulatorLink


def on_link(simulator, act):
    """`act(link)` on the simulator's socket, or on the device without one."""
    try:
        with SimulatorLink(simulator) if simulator else usb.open_link() as link:
            act(link)
    except usb.NoDevice as error:
        sys.exit(str(error))
    except SerialException as error:
        sys.exit(usb.explain(error))
    except NoSimulator as error:
        sys.exit(str(error))
    except ConsoleError as error:
        sys.exit(f'The device: {error}')
