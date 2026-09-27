import unittest

from cute_display_installer.rtc import NoRegisters, describe, registers_in

# 2026-09-26 11:46:44; alarm 1 at 07:30:00 every day, on and raised; alarm 2 unused.
REGISTERS = bytes([0x44, 0x46, 0x11, 0x06, 0x26, 0x09, 0x26,
                   0x00, 0x30, 0x07, 0x80,
                   0x80, 0x80, 0x80,
                   0x1d, 0x01, 0x00, 0x19, 0x40])


class Registers(unittest.TestCase):
    def test_they_are_found_in_the_hardware_test_log(self):
        line = 'I (1520) hwtest: DS3231 registers: ' + ' '.join(f'{b:02x}' for b in REGISTERS) + '\n'
        self.assertEqual(registers_in(line), REGISTERS)
        self.assertIsNone(registers_in('I (1500) hwtest: cute-display hardware test\n'))

    def test_a_silent_rtc_is_said(self):
        with self.assertRaisesRegex(NoRegisters, 'did not answer the hardware test: I2C timeout'):
            registers_in('W (1520) hwtest: DS3231 registers: I2C timeout\n')

    def test_they_read_as_the_time_the_alarms_and_the_flags(self):
        self.assertEqual(describe(REGISTERS), [
            'time     2026-09-26 11:46:44',
            'alarm 1  07:30:00 day 0, mask bits ...x, on, raised',
            'alarm 2  00:00 day 0, mask bits xxx, off, not raised',
            'control 0x1d  status 0x01  aging 0x00',
        ])


if __name__ == '__main__':
    unittest.main()
