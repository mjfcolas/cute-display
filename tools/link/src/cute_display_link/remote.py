"""`remote`: tap, hold and turn the controls of the device, or of a simulator, from this
computer; see its lights and its speaker; read and set its clock."""
import argparse
import sys
from datetime import datetime

from serial import SerialException

from . import clock, controls, observation, usb
from .console import ConsoleError
from .simulator import NoSimulator, SimulatorLink


def show_clock(link, arguments):
    if arguments.set:
        clock.set_to(link, arguments.set if arguments.set.tzinfo else arguments.set.astimezone())
    print(clock.read(link).isoformat())


def main():
    parser = argparse.ArgumentParser(prog='remote', description=__doc__)
    parser.add_argument('--simulator', metavar='SOCKET', help="a simulator's, started with --console SOCKET")
    commands = parser.add_subparsers(required=True, metavar='command')

    tap = commands.add_parser('tap', help='press and release a button')
    tap.add_argument('button', choices=controls.BUTTONS)
    tap.set_defaults(act=lambda link, a: controls.tap(link, a.button))

    hold = commands.add_parser('hold', help='hold buttons down together, then release them')
    hold.add_argument('milliseconds', type=int)
    hold.add_argument('buttons', nargs='+', choices=controls.BUTTONS)
    hold.set_defaults(act=lambda link, a: controls.hold_until_released(link, a.milliseconds, a.buttons))

    turn = commands.add_parser('turn', help='turn the wheel')
    turn.add_argument('clockwise_detents', type=int)
    turn.set_defaults(act=lambda link, a: controls.turn(link, a.clockwise_detents))

    commands.add_parser('lights', help='the front light and the reading lamp').set_defaults(
        act=lambda link, a: print('front light {} %, reading lamp {} %'.format(*observation.lights(link))))
    commands.add_parser('sound', help='whether the speaker plays').set_defaults(
        act=lambda link, a: print('playing' if observation.is_playing(link) else 'silent'))
    rtc = commands.add_parser('clock', help="the RTC's time, UTC")
    rtc.add_argument('--set', metavar='TIME', type=datetime.fromisoformat, help='set it first: ISO 8601, local without a zone')
    rtc.set_defaults(act=show_clock)

    arguments = parser.parse_args()
    try:
        with SimulatorLink(arguments.simulator) if arguments.simulator else usb.open_link() as link:
            arguments.act(link, arguments)
    except usb.NoDevice as error:
        sys.exit(str(error))
    except SerialException as error:
        sys.exit(usb.explain(error))
    except NoSimulator as error:
        sys.exit(str(error))
    except ConsoleError as error:
        sys.exit(f'The device: {error}')
