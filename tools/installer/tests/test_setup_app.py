import unittest

from textual.widgets import Button, Input, OptionList, SelectionList

from cute_display_installer.config.airports import Airport
from cute_display_installer.config.place import Place
from cute_display_installer.config.places import Found
from cute_display_installer.config.wifi import Wifi
from cute_display_installer.flash.layout import AppImage, Security, Slot
from cute_display_installer.setup.answers import Answers
from cute_display_installer.setup.app import SetupApp
from cute_display_installer.setup.clock import Failed
from cute_display_installer.setup.screens.airports import AirportsScreen
from cute_display_installer.setup.screens.apps import AppsScreen
from cute_display_installer.setup.screens.done import DoneScreen
from cute_display_installer.setup.screens.install import InstallScreen
from cute_display_installer.setup.screens.message import Message, Tone
from cute_display_installer.setup.screens.place import PlaceScreen
from cute_display_installer.setup.screens.ringtones import RingtonesScreen
from cute_display_installer.setup.screens.state import StateScreen
from cute_display_installer.setup.screens.summary import SummaryScreen
from cute_display_installer.setup.screens.time_zone import TimeZoneScreen
from cute_display_installer.setup.screens.wifi import WifiScreen
from test_layout import HABITY_1_1_1, habity, header, slots

LYON = Found(Place('Lyon', 45.7485, 4.8467), 'Auvergne-Rhône-Alpes, France', 'Europe/Paris')
AIRPORTS = [Airport('LFLY', 45.7272, 4.9444, 'Lyon Bron Airport', False, 8.0),
            Airport('LFLL', 45.7256, 5.0811, 'Lyon Saint-Exupéry Airport', True, 18.3)]
LATEST = header('cute-display', '2026.9.0')
RUNNING = habity(booting=Slot.APP1, slots=slots(HABITY_1_1_1, AppImage('cute-display', '2026.9.0')))


class FakeClock:
    """A clock holding `unit` and a card with `answers`, remembering what it was asked."""

    def __init__(self, unit=RUNNING, answers=None, unplugged=False):
        self.unit, self.answers, self.unplugged = unit, answers or Answers(), unplugged
        self.done, self.written, self.copied = [], [], []

    def read(self):
        self.done.append('read')
        return self.unit

    def back_up(self, folder):
        self.done.append('back up')
        return f'{folder}/habity-flash.bin'

    def latest(self):
        return LATEST

    def install(self, contents):
        self.done.append('install')

    def start(self, slot):
        self.done.append(f'start {slot}')

    def wait_for_cute_display(self):
        self.done.append('wait')

    def read_card(self):
        return self.answers

    def restart(self):
        self.done.append('restart')

    def write_card(self, files, copies):
        if self.unplugged:
            raise Failed('No Habity found on USB.')
        self.written.append(files)
        self.copied.append(copies)


def setup_app(clock):
    return SetupApp(clock, search=lambda name: [LYON] if name == 'Lyon' else [], airports_around=lambda place: AIRPORTS)


async def settle(pilot):
    await pilot.app.workers.wait_for_complete()
    await pilot.pause()


async def press(pilot, button_id):
    await pilot.click(f'#{button_id}')
    await settle(pilot)


async def type_into(pilot, selector, text):
    pilot.app.screen.query_one(selector, Input).value = text
    pilot.app.screen.query_one(selector, Input).focus()
    await pilot.press('enter')
    await pilot.pause()


async def to_the_questions(pilot):
    """From the start, the backup skipped, on a clock running an up-to-date Cute Display."""
    await settle(pilot)
    await press(pilot, 'no')
    await press(pilot, 'next')
    await press(pilot, 'done')
    await press(pilot, 'yes')


class FirstPart(unittest.IsolatedAsyncioTestCase):
    async def test_a_new_clock_is_backed_up_installed_then_set_up(self):
        clock = FakeClock(unit=habity())
        app = setup_app(clock)
        async with app.run_test() as pilot:
            await settle(pilot)
            await press(pilot, 'yes')
            self.assertIn('habity-flash.bin', str(app.screen.query_one(Message).render()))
            await press(pilot, 'next')
            self.assertIsInstance(app.screen, StateScreen)
            self.assertIn('into app1', str(app.screen.query_one(Message).render()))
            await press(pilot, 'next')
            self.assertIsInstance(app.screen, InstallScreen)
            self.assertIn('Install Cute Display 2026.9.0 into app1?', str(app.screen.query_one(Message).render()))
            await press(pilot, 'install')
            await press(pilot, 'done')
            await press(pilot, 'yes')
            self.assertIsInstance(app.screen, WifiScreen)
        self.assertEqual(clock.done, ['back up', 'read', 'install', 'wait'])

    async def test_an_up_to_date_cute_display_goes_straight_to_the_questions(self):
        clock = FakeClock()
        app = setup_app(clock)
        async with app.run_test() as pilot:
            await to_the_questions(pilot)
            self.assertIsInstance(app.screen, WifiScreen)
        self.assertEqual(clock.done, ['read'])

    async def test_an_installed_cute_display_is_said_where_it_is(self):
        app = setup_app(FakeClock())
        async with app.run_test() as pilot:
            await settle(pilot)
            await press(pilot, 'no')
            self.assertIn('Cute Display is in app1.', str(app.screen.query_one(Message).render()))

    async def test_a_cute_display_that_does_not_run_is_started(self):
        clock = FakeClock(unit=habity(booting=Slot.APP0, slots=slots(HABITY_1_1_1, AppImage('cute-display', '2026.9.0'))))
        app = setup_app(clock)
        async with app.run_test() as pilot:
            await settle(pilot)
            await press(pilot, 'no')
            await press(pilot, 'next')
            await press(pilot, 'start')
            await press(pilot, 'done')
            await press(pilot, 'yes')
            self.assertIsInstance(app.screen, WifiScreen)
        self.assertEqual(clock.done, ['read', 'start app1', 'wait'])

    async def test_a_clock_that_cannot_take_cute_display_stops_at_its_state(self):
        app = setup_app(FakeClock(unit=habity(security=Security(True, False))))
        async with app.run_test() as pilot:
            await settle(pilot)
            await press(pilot, 'no')
            self.assertIn('secure boot', str(app.screen.query_one(Message).render()))
            self.assertEqual([button.id for button in app.screen.query(Button)], ['quit'])

    async def test_not_installing_ends_without_setting_up(self):
        clock = FakeClock(unit=habity())
        app = setup_app(clock)
        async with app.run_test() as pilot:
            await settle(pilot)
            await press(pilot, 'no')
            await press(pilot, 'next')
            await press(pilot, 'done')
            self.assertIsInstance(app.screen, DoneScreen)
        self.assertEqual(clock.done, ['read'])


class Questions(unittest.IsolatedAsyncioTestCase):
    async def test_a_new_card_is_set_up_screen_by_screen(self):
        clock = FakeClock()
        app = setup_app(clock)
        async with app.run_test() as pilot:
            await to_the_questions(pilot)
            await type_into(pilot, '#ssid', 'Home')
            await type_into(pilot, '#password', 's3cret')
            self.assertIsInstance(app.screen, PlaceScreen)

            await type_into(pilot, '#search', 'Lyon')
            await app.workers.wait_for_complete()
            await pilot.pause()
            await pilot.press('enter')
            await pilot.pause()
            self.assertIsInstance(app.screen, TimeZoneScreen)
            self.assertEqual(app.screen.query_one('#filter', Input).value, 'Europe/Paris')

            await pilot.click(Button)
            self.assertIsInstance(app.screen, AppsScreen)
            self.assertEqual(app.screen.query_one(SelectionList).selected, ['alarm', 'weather', 'radar'])

            await pilot.click(Button)
            self.assertIsInstance(app.screen, AirportsScreen)
            await app.workers.wait_for_complete()
            await pilot.pause()
            self.assertEqual(app.screen.query_one(SelectionList).selected, ['LFLL'])

            await pilot.click(Button)
            self.assertIsInstance(app.screen, SummaryScreen)
            await pilot.click(Button)
            await settle(pilot)
            self.assertIsInstance(app.screen, DoneScreen)
            message = app.screen.query_one(Message)
            self.assertTrue(message.has_class(Tone.DONE.value))
            self.assertIn('Written, and the clock restarted', str(message.render()))

        self.assertEqual(app.answers, Answers(Wifi('Home', 's3cret'), LYON.place, 'Europe/Paris', ['LFLL']))
        self.assertEqual(clock.written, [app.answers.files(AIRPORTS)])
        self.assertEqual(clock.done[-1], 'restart')

    async def test_the_order_of_the_apps_on_the_card_is_kept(self):
        known = Answers(Wifi('Home', 's3cret'), LYON.place, 'Europe/Paris', ['LFLY'], ['radar', 'alarm'])
        app = setup_app(FakeClock(answers=known))
        async with app.run_test() as pilot:
            await to_the_questions(pilot)
            await type_into(pilot, '#password', 's3cret')
            await pilot.click(Button)
            await pilot.click(Button)
            self.assertIsInstance(app.screen, AppsScreen)
            app.screen.query_one(SelectionList).select('weather')
            await pilot.click(Button)
        self.assertEqual(app.answers.apps, ['radar', 'alarm', 'weather'])

    async def test_without_the_radar_its_airports_are_not_asked_nor_written(self):
        known = Answers(Wifi('Home', 's3cret'), LYON.place, 'Europe/Paris', ['LFLY'])
        clock = FakeClock(answers=known)
        app = setup_app(clock)
        async with app.run_test() as pilot:
            await to_the_questions(pilot)
            await type_into(pilot, '#password', 's3cret')
            await pilot.click(Button)
            await pilot.click(Button)
            self.assertIsInstance(app.screen, AppsScreen)
            app.screen.query_one(SelectionList).deselect('radar')
            await pilot.click(Button)
            self.assertIsInstance(app.screen, SummaryScreen)
            await pilot.click(Button)
            await settle(pilot)
            self.assertIsInstance(app.screen, DoneScreen)

        self.assertEqual(app.answers.apps, ['alarm', 'weather'])
        self.assertEqual(sorted(clock.written[0]), ['cute-display/general.conf', 'cute-display/wifi.conf'])

    async def test_habitys_ringtones_are_offered_after_the_apps_and_copied(self):
        known = Answers(Wifi('Home', 's3cret'), LYON.place, 'Europe/Paris', ['LFLY'], ['alarm'], ['Zen.mp3', 'Default.mp3'])
        clock = FakeClock(answers=known)
        app = setup_app(clock)
        async with app.run_test() as pilot:
            await to_the_questions(pilot)
            await type_into(pilot, '#password', 's3cret')
            await pilot.click(Button)
            await pilot.click(Button)
            await pilot.click(Button)
            self.assertIsInstance(app.screen, RingtonesScreen)
            self.assertIn('Zen, Default', str(app.screen.query_one('#body').render()))
            await press(pilot, 'copy')
            self.assertIsInstance(app.screen, SummaryScreen)
            await pilot.click(Button)
            await settle(pilot)
            self.assertIsInstance(app.screen, DoneScreen)
        self.assertEqual(clock.copied, [{'sounds/alarm/Zen.mp3': 'cute-display/apps/alarm/ringtones/Zen.mp3',
                                         'sounds/alarm/Default.mp3': 'cute-display/apps/alarm/ringtones/Default.mp3'}])

    async def test_habitys_ringtones_declined_are_not_copied(self):
        known = Answers(Wifi('Home', 's3cret'), LYON.place, 'Europe/Paris', ['LFLY'], ['alarm'], ['Zen.mp3'])
        clock = FakeClock(answers=known)
        app = setup_app(clock)
        async with app.run_test() as pilot:
            await to_the_questions(pilot)
            await type_into(pilot, '#password', 's3cret')
            await pilot.click(Button)
            await pilot.click(Button)
            await pilot.click(Button)
            await press(pilot, 'no')
            await pilot.click(Button)
            await settle(pilot)
        self.assertEqual(clock.copied, [{}])

    async def test_a_wifi_the_device_cannot_keep_is_refused_on_the_spot(self):
        app = setup_app(FakeClock())
        async with app.run_test() as pilot:
            await to_the_questions(pilot)
            await type_into(pilot, '#ssid', 'x' * 33)
            await type_into(pilot, '#password', '')
            self.assertIn('32 bytes', str(app.screen.query_one(Message).render()))

    async def test_a_card_gone_before_writing_is_said_and_can_be_tried_again(self):
        known = Answers(Wifi('Home', 's3cret'), LYON.place, 'Europe/Paris', ['LFLY'])
        app = setup_app(FakeClock(answers=known, unplugged=True))
        async with app.run_test() as pilot:
            await to_the_questions(pilot)
            await type_into(pilot, '#password', 's3cret')
            await pilot.click(Button)
            await pilot.click(Button)
            await pilot.click(Button)
            await app.workers.wait_for_complete()
            await pilot.pause()
            await pilot.click(Button)
            await pilot.click(Button)
            await app.workers.wait_for_complete()
            await pilot.pause()
            self.assertIsInstance(app.screen, SummaryScreen)
            self.assertIn('Could not write: No Habity found', str(app.screen.query_one(Message).render()))
            self.assertTrue(app.screen.query_one(Message).has_class(Tone.PROBLEM.value))
            self.assertFalse(app.screen.query_one(Button).disabled)

    async def test_what_the_card_holds_is_kept_by_going_on(self):
        known = Answers(Wifi('Home', 's3cret'), LYON.place, 'Europe/Paris', ['LFLY'])
        app = setup_app(FakeClock(answers=known))
        async with app.run_test() as pilot:
            await to_the_questions(pilot)
            await type_into(pilot, '#password', 's3cret')
            await pilot.click(Button)
            await pilot.click(Button)
            await pilot.click(Button)
            await app.workers.wait_for_complete()
            await pilot.pause()
            self.assertEqual(app.screen.query_one(SelectionList).selected, ['LFLY'])
            await pilot.press('escape')
            self.assertIsInstance(app.screen, AppsScreen)
            self.assertEqual(app.screen.query_one(SelectionList).selected, ['alarm', 'weather', 'radar'])
            await pilot.press('escape')
            self.assertIsInstance(app.screen, TimeZoneScreen)
            self.assertEqual(len(app.screen.query_one(OptionList).options), 1)


if __name__ == '__main__':
    unittest.main()
