#!/usr/bin/env python3
"""Copy the DS3231's registers into a file.

  tools/rtc_backup.py <local file>

The device must run the hardware test image, which logs them as it starts; this resets
it and reads the log. The other end is src/firmware/src/bin/hwtest.rs.
"""
import subprocess
import sys
import time

import serial

from sd import open_port

MARK = 'DS3231 registers: '
LOG_TIMEOUT_S = 20
DAYS = ['', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']


def bcd(b):
    return (b >> 4) * 10 + (b & 0x0f)


def alarm(registers, first, has_seconds):
    fields = registers[first:first + (4 if has_seconds else 3)]
    seconds = f':{bcd(fields[0] & 0x7f):02d}' if has_seconds else ''
    minute, hour, day = fields[-3:]
    masks = ''.join('x' if f & 0x80 else '.' for f in fields)
    when = (f'{DAYS[day & 0x07]}' if day & 0x40 else f'day {bcd(day & 0x3f)}')
    return f'{bcd(hour & 0x3f):02d}:{bcd(minute & 0x7f):02d}{seconds} {when}, mask bits {masks}'


def describe(r):
    control, status = r[0x0e], r[0x0f]
    print(f'time     20{bcd(r[6]):02d}-{bcd(r[5] & 0x1f):02d}-{bcd(r[4]):02d} '
          f'{bcd(r[2] & 0x3f):02d}:{bcd(r[1]):02d}:{bcd(r[0]):02d}')
    print(f'alarm 1  {alarm(r, 0x07, True)}, {"on" if control & 1 else "off"}, '
          f'{"raised" if status & 1 else "not raised"}')
    print(f'alarm 2  {alarm(r, 0x0b, False)}, {"on" if control & 2 else "off"}, '
          f'{"raised" if status & 2 else "not raised"}')
    print(f'control 0x{control:02x}  status 0x{status:02x}  aging 0x{r[0x10]:02x}')


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    subprocess.run(['espflash', 'reset'], check=True, stdout=subprocess.DEVNULL)
    deadline = time.monotonic() + LOG_TIMEOUT_S
    while time.monotonic() < deadline:
        try:
            link = open_port()
        except (serial.SerialException, SystemExit):
            time.sleep(0.2)
            continue
        with link:
            while time.monotonic() < deadline:
                line = link.readline().decode(errors='replace')
                if MARK in line:
                    registers = bytes.fromhex(line.split(MARK, 1)[1].strip())
                    with open(sys.argv[1], 'wb') as f:
                        f.write(registers)
                    print(f'{len(registers)} registers -> {sys.argv[1]}')
                    describe(registers)
                    return
    sys.exit('the registers never showed in the log; is the hardware test image running?')


if __name__ == '__main__':
    main()
