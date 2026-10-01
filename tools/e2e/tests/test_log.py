import unittest

from cute_display_e2e.log import Log


class Crashes(unittest.TestCase):
    def test_a_panic_or_a_reboot_is_a_crash_and_the_rest_is_not(self):
        log = Log()
        for line in ["[INFO  app::network] weather: updating", "thread 'ui' panicked at src/app/src/lib.rs:9:5:",
                     '\x1b[0;31mE (812) task_wdt: Rebooting...\x1b[0m']:
            log.add(line)
        self.assertEqual(log.crashes(), ["thread 'ui' panicked at src/app/src/lib.rs:9:5:", 'E (812) task_wdt: Rebooting...'])
