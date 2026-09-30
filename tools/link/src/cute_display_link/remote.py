"""`remote`: tap, hold and turn the controls of the device, or of a simulator, from this
computer."""
import argparse
import sys

from serial import SerialException

from . import controls, usb
from .console import ConsoleError
from .simulator import NoSimulator, SimulatorLink


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
