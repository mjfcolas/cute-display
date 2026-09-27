"""Where the device is plugged: the ESP32-S3's own USB serial port, on any computer."""
import os

import serial
from serial.tools import list_ports

ESPRESSIF_USB_JTAG = (0x303a, 0x1001)
PORT_VARIABLE = 'CUTE_DISPLAY_PORT'


class NoDevice(Exception):
    pass


def port(ports=None):
    """The device's port: CUTE_DISPLAY_PORT, else the one Espressif USB port plugged in."""
    chosen = os.environ.get(PORT_VARIABLE)
    if chosen:
        return chosen
    found = [p.device for p in (ports if ports is not None else list_ports.comports())
             if (p.vid, p.pid) == ESPRESSIF_USB_JTAG]
    if not found:
        raise NoDevice('No Habity found on USB. Is the cable plugged in, and does it carry data, '
                       'not only power?')
    if len(found) > 1:
        raise NoDevice(f'Several devices found ({", ".join(found)}): choose one with {PORT_VARIABLE}.')
    return found[0]


def open_link():
    """A line to the running firmware's console."""
    # Opening the port raises DTR and RTS, and pyserial's defaults keep them there. On the
    # ESP32-S3, lowering one before the other is what resets the chip.
    link = serial.Serial(port(), 115200, timeout=0.2)
    link.reset_input_buffer()
    return link
