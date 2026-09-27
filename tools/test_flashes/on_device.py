"""The installer on the device, one test-bins/ image after another: each is written the
way docs/troubleshooting.md puts a backup back, then `cute-display` is run on it as
people run it, and the device is read back.

  just test-bins <backup>    first, then
  just test-on-device

The device ends as `updated-once` with Cute Display installed: app1 booting it. Each
image is a whole flash, NVS included: Habity's settings and Wi-Fi go back to the backup's.
"""
import os
import re
import subprocess
import sys
import time
import unittest

from cute_display_installer import usb
from cute_display_installer.flash.layout import APP_HEADER_SIZE, app_image

HERE = os.path.dirname(os.path.abspath(__file__))
IMAGES = os.path.join(HERE, 'test-bins')
APP = os.path.join(HERE, '..', '..', 'src', 'firmware', 'target', 'cute-display-app.bin')
PUT_BACK = ['uvx', '--quiet', '--from', 'esptool>=5.1,<6', 'esptool', '--chip', 'esp32s3', 'write-flash', '0x0']
LOG_TIMEOUT_S = 30


def _ours():
    with open(APP, 'rb') as f:
        return str(app_image(f.read(APP_HEADER_SIZE)))


OURS = _ours()
SLOT_LINE = re.compile(r'^\s+(factory|app0|app1)\s+(.*?)(\s+<- boots)?$')


def put_back(case):
    subprocess.run(PUT_BACK + [os.path.join(IMAGES, f'{case}.bin')], check=True, capture_output=True)


def cute_display(*arguments, answer=''):
    """The command as people run it: what it printed, and whether it succeeded."""
    done = subprocess.run(['cute-display', *arguments], input=answer, capture_output=True, text=True)
    return done.stdout + done.stderr, done.returncode == 0


def slots():
    """What `check` says each slot holds, and which one boots."""
    printed, _ = cute_display('check')
    held, booting = {}, None
    for line in printed.splitlines():
        found = SLOT_LINE.match(line)
        if found:
            held[found[1]] = found[2]
            booting = found[1] if found[3] else booting
    return held, booting


def starts_cute_display():
    """Whether the device, restarted once its port is open, logs Cute Display's first line."""
    deadline = time.monotonic() + LOG_TIMEOUT_S
    while time.monotonic() < deadline:
        try:
            link = usb.open_link()
        except (OSError, usb.NoDevice):
            time.sleep(0.2)
            continue
        with link:
            # RTS raised then lowered, DTR low: the USB-Serial/JTAG resets the chip into its
            # firmware, as esptool's hard reset does, so the log is read from its start.
            link.dtr, link.rts = False, True
            link.rts = False
            while time.monotonic() < deadline:
                if 'app: Cute Display' in link.readline().decode(errors='replace'):
                    return True
    return False


class OnDevice(unittest.TestCase):
    case = None

    @classmethod
    def setUpClass(cls):
        if cls.case:
            put_back(cls.case)

    def assert_slots(self, held, booting):
        self.assertEqual(slots(), (held, booting))


class UpdatedOnce(OnDevice):
    case = 'updated-once'

    def test_cute_display_goes_into_the_empty_slot_and_habity_comes_back(self):
        printed, ok = cute_display('check')
        self.assertTrue(ok, printed)
        self.assertIn('into app1.', printed)
        printed, ok = cute_display('install', APP)
        self.assertTrue(ok, printed)
        self.assertTrue(starts_cute_display())
        self.assert_slots({'factory': 'Habity 1.1.0', 'app0': 'Habity 1.1.1', 'app1': OURS}, 'app1')
        cute_display('boot', 'habity')
        self.assertEqual(slots()[1], 'app0')
        cute_display('boot', 'cute-display')
        self.assertTrue(starts_cute_display())


class NeverUpdated(OnDevice):
    case = 'never-updated'

    def test_cute_display_goes_into_app0_and_habity_is_the_factory_one(self):
        printed, _ = cute_display('check')
        self.assertIn('into app0.', printed)
        printed, ok = cute_display('install', APP)
        self.assertTrue(ok, printed)
        self.assertTrue(starts_cute_display())
        self.assert_slots({'factory': 'Habity 1.1.0', 'app0': OURS, 'app1': 'empty'}, 'app0')
        cute_display('boot', 'habity')
        self.assertEqual(slots()[1], 'factory')


class UpdatedTwice(OnDevice):
    case = 'updated-twice'

    def test_the_older_habity_is_erased_only_once_confirmed(self):
        printed, _ = cute_display('check')
        self.assertIn('into app0, erasing Habity 1.1.1 there, the older of the two.', printed)
        printed, ok = cute_display('install', APP, answer='n\n')
        self.assertFalse(ok)
        self.assertIn('not confirmed', printed)
        self.assert_slots({'factory': 'Habity 1.1.0', 'app0': 'Habity 1.1.1', 'app1': 'Habity 1.1.2'}, 'app1')
        printed, ok = cute_display('install', APP, answer='y\n')
        self.assertTrue(ok, printed)
        self.assertTrue(starts_cute_display())
        self.assert_slots({'factory': 'Habity 1.1.0', 'app0': OURS, 'app1': 'Habity 1.1.2'}, 'app0')
        cute_display('boot', 'habity')
        self.assertEqual(slots()[1], 'app1')


class Refused(OnDevice):
    """Cases the installer refuses: nothing is written."""

    def refuses(self, case, reason):
        put_back(case)
        before = slots()
        for command in (['check'], ['install', APP]):
            printed, ok = cute_display(*command)
            self.assertFalse(ok, printed)
            self.assertIn(reason, printed)
        self.assertEqual(slots(), before)

    def test_an_unreadable_version(self):
        self.refuses('updated-twice-unreadable', 'which one is older cannot be told')

    def test_another_partition_table(self):
        self.refuses('other-partition-table', 'the partition table is not the one this tool knows')

    def test_no_habity_left(self):
        self.refuses('no-habity', 'there would be no way back')
        printed, ok = cute_display('boot', 'habity')
        self.assertFalse(ok)
        self.assertIn('no habity to start', printed)


class CuteDisplayInApp0(OnDevice):
    case = 'cute-display-in-app0'

    def test_an_update_stays_in_its_slot(self):
        printed, _ = cute_display('check')
        self.assertIn('into app0.', printed)
        printed, ok = cute_display('install', APP)
        self.assertTrue(ok, printed)
        self.assertTrue(starts_cute_display())
        self.assert_slots({'factory': 'Habity 1.1.0', 'app0': OURS, 'app1': 'Habity 1.1.2'}, 'app0')
        cute_display('boot', 'habity')
        self.assertEqual(slots()[1], 'app1')


def tearDownModule():
    put_back('updated-once')
    printed, ok = cute_display('install', APP)
    if not ok:
        raise RuntimeError(f'the device was left without Cute Display:\n{printed}')


if __name__ == '__main__':
    if not os.path.exists(os.path.join(IMAGES, 'updated-once.bin')):
        sys.exit('No test images: `just test-bins <backup>` first.')
    unittest.main(verbosity=2)
