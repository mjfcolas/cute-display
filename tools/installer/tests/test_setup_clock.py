import unittest
from contextlib import ExitStack, nullcontext
from unittest import mock
from urllib.error import URLError

from esptool.cmds import FatalError

from cute_display_link import usb
from cute_display_link.console import ConsoleError

from cute_display_installer import releases
from cute_display_installer.flash.layout import Placement, Security, Slot
from cute_display_installer.setup import clock
from cute_display_installer.setup.clock import Failed, UsbClock
from test_layout import OURS, habity, header

IMAGE = header(OURS.project, OURS.version)


def device_reading_as(unit):
    """The flash module's hardware calls, the device reading as `unit`."""
    flash = mock.Mock()
    flash.connect.return_value = nullcontext('esp')
    flash.read_device.return_value = unit
    return [mock.patch.object(clock.device, name, getattr(flash, name))
            for name in ('connect', 'read_device', 'restart', 'install')], flash


class Messages(unittest.TestCase):
    def test_each_expected_failure_says_what_to_do(self):
        said = {
            usb.NoDevice('No Habity found on USB.'): 'No Habity found',
            FatalError('Failed to connect'): 'Unplug it',
            ConsoleError('no answer'): 'SD card: no answer',
            releases.ReleaseError('no release yet'): 'GitHub: no release yet.',
            URLError('Name or service not known'): 'is this computer online?',
            TimeoutError('timed out'): 'is this computer online?',
        }
        for error, text in said.items():
            self.assertIn(text, clock._said(error), error)

    def test_anything_else_is_not_hidden(self):
        self.assertIsNone(clock._said(KeyError('bug')))
        with self.assertRaises(KeyError):
            clock._step(lambda: {}['bug'])


class Install(unittest.TestCase):
    def install(self, unit, contents=IMAGE):
        patches, flash = device_reading_as(unit)
        with ExitStack() as stack:
            for patch in patches:
                stack.enter_context(patch)
            try:
                UsbClock().install(contents)
            except Failed as failed:
                return flash, str(failed)
        return flash, None

    def test_a_clock_that_may_take_it_gets_it_where_it_goes(self):
        flash, failed = self.install(habity())
        self.assertIsNone(failed)
        flash.install.assert_called_once_with('esp', Placement(Slot.APP1), Slot.APP0, IMAGE)

    def test_a_refused_clock_is_restarted_with_nothing_written(self):
        flash, failed = self.install(habity(security=Security(True, False)))
        self.assertIn('Cannot install: secure boot', failed)
        flash.install.assert_not_called()
        flash.restart.assert_called_once_with('esp')

    def test_an_image_that_is_not_cute_display_is_not_written(self):
        flash, failed = self.install(habity(), contents=header('habity', '1.1.2'))
        self.assertIn("it is not a Cute Display image", failed)
        flash.install.assert_not_called()


class Latest(unittest.TestCase):
    def setUp(self):
        patched = mock.patch.object(clock.versions, 'this_installer_version', return_value='2026.9.1')
        patched.start()
        self.addCleanup(patched.stop)

    def test_a_release_whose_image_is_not_cute_display_is_refused(self):
        with mock.patch.object(clock.releases, 'latest_release',
                               return_value=releases.Release('x.bin', header('habity', '1.1.2'), ())):
            with self.assertRaisesRegex(Failed, 'latest release cannot be installed'):
                UsbClock().latest()

    def test_a_release_this_installer_version_is_too_old_for_is_refused(self):
        with mock.patch.object(clock.releases, 'latest_release',
                               return_value=releases.Release('x.bin', IMAGE, (), '2026.10.0')):
            with self.assertRaisesRegex(Failed, 'needs installer 2026.10.0 or newer'):
                UsbClock().latest()

    def test_a_cute_display_release_comes_through(self):
        release = releases.Release('x.bin', IMAGE, ())
        with mock.patch.object(clock.releases, 'latest_release', return_value=release):
            self.assertEqual(UsbClock().latest(), release)


class Waiting(unittest.TestCase):
    def test_the_console_is_asked_again_until_it_answers(self):
        attempts = [usb.NoDevice('restarting'), ConsoleError('no answer'), None]

        def open_link():
            failure = attempts.pop(0)
            if failure:
                raise failure
            return nullcontext('link')

        with mock.patch.object(clock.usb, 'open_link', open_link), \
                mock.patch.object(clock, 'entries') as entries, mock.patch.object(clock.time, 'sleep'):
            UsbClock().wait_for_cute_display()
        entries.assert_called_once_with('link')

    def test_a_clock_that_never_answers_is_given_up_on(self):
        now = iter(range(0, 1000, 10))
        with mock.patch.object(clock.usb, 'open_link', side_effect=usb.NoDevice('No Habity found on USB.')), \
                mock.patch.object(clock.time, 'monotonic', lambda: next(now)), \
                mock.patch.object(clock.time, 'sleep') as sleep:
            with self.assertRaisesRegex(Failed, 'No Habity found'):
                UsbClock().wait_for_cute_display()
        self.assertEqual(sleep.call_count, clock.STARTING_TIMEOUT_S // 10)


if __name__ == '__main__':
    unittest.main()
