"""The DS3231's registers, as the hardware test image logs them when it starts
(src/firmware/src/bin/hwtest.rs)."""

MARK = 'DS3231 registers: '
DAYS = ['', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']
SECONDS, MINUTES, HOURS, DATE, MONTH, YEAR = 0x00, 0x01, 0x02, 0x04, 0x05, 0x06
ALARM_1, ALARM_2 = 0x07, 0x0b
CONTROL, STATUS, AGING = 0x0e, 0x0f, 0x10


class NoRegisters(Exception):
    """The hardware test could not read them: the log says why instead."""


def registers_in(line):
    """The registers a log line carries; None for any other line."""
    if MARK not in line:
        return None
    logged = line.split(MARK, 1)[1].strip()
    try:
        return bytes.fromhex(logged)
    except ValueError:
        raise NoRegisters(f'The RTC did not answer the hardware test: {logged}') from None


def _bcd(b):
    return (b >> 4) * 10 + (b & 0x0f)


def _alarm(registers, first, has_seconds):
    fields = registers[first:first + (4 if has_seconds else 3)]
    seconds = f':{_bcd(fields[0] & 0x7f):02d}' if has_seconds else ''
    minute, hour, day = fields[-3:]
    masks = ''.join('x' if f & 0x80 else '.' for f in fields)
    when = DAYS[day & 0x07] if day & 0x40 else f'day {_bcd(day & 0x3f)}'
    return f'{_bcd(hour & 0x3f):02d}:{_bcd(minute & 0x7f):02d}{seconds} {when}, mask bits {masks}'


def describe(registers):
    control, status = registers[CONTROL], registers[STATUS]
    return [
        f'time     20{_bcd(registers[YEAR]):02d}-{_bcd(registers[MONTH] & 0x1f):02d}-{_bcd(registers[DATE]):02d} '
        f'{_bcd(registers[HOURS] & 0x3f):02d}:{_bcd(registers[MINUTES]):02d}:{_bcd(registers[SECONDS]):02d}',
        f'alarm 1  {_alarm(registers, ALARM_1, True)}, {"on" if control & 1 else "off"}, '
        f'{"raised" if status & 1 else "not raised"}',
        f'alarm 2  {_alarm(registers, ALARM_2, False)}, {"on" if control & 2 else "off"}, '
        f'{"raised" if status & 2 else "not raised"}',
        f'control 0x{control:02x}  status 0x{status:02x}  aging 0x{registers[AGING]:02x}',
    ]
