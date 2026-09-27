import unittest

from textual.widgets import Button, Input, OptionList, SelectionList

from cute_display_installer.config.airports import Airport
from cute_display_installer.config.place import Place
from cute_display_installer.config.places import Found
from cute_display_installer.config.wifi import Wifi
from cute_display_installer.setup.answers import Answers
from cute_display_installer.setup.card import Unreachable
from cute_display_installer.setup.app import SetupApp
from cute_display_installer.setup.screens.airports import AirportsScreen
from cute_display_installer.setup.screens.message import Message, Tone
from cute_display_installer.setup.screens.place import PlaceScreen
from cute_display_installer.setup.screens.summary import SummaryScreen
from cute_display_installer.setup.screens.time_zone import TimeZoneScreen

LYON = Found(Place('Lyon', 45.7485, 4.8467), 'Auvergne-Rhône-Alpes, France', 'Europe/Paris')
AIRPORTS = [Airport('LFLY', 45.7272, 4.9444, 'Lyon Bron Airport', False, 8.0),
            Airport('LFLL', 45.7256, 5.0811, 'Lyon Saint-Exupéry Airport', True, 18.3)]


def setup_app(answers):
    written = []
    app = SetupApp(answers, search=lambda name: [LYON] if name == 'Lyon' else [],
                   airports_around=lambda place: AIRPORTS, write=written.append)
    return app, written


async def type_into(pilot, selector, text):
    pilot.app.screen.query_one(selector, Input).value = text
    pilot.app.screen.query_one(selector, Input).focus()
    await pilot.press('enter')
    await pilot.pause()


class Setup(unittest.IsolatedAsyncioTestCase):
    async def test_a_new_card_is_set_up_screen_by_screen(self):
        app, written = setup_app(Answers())
        async with app.run_test() as pilot:
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
            self.assertIsInstance(app.screen, AirportsScreen)
            await app.workers.wait_for_complete()
            await pilot.pause()
            self.assertEqual(app.screen.query_one(SelectionList).selected, ['LFLL'])

            await pilot.click(Button)
            self.assertIsInstance(app.screen, SummaryScreen)
            await pilot.click(Button)
            await app.workers.wait_for_complete()
            await pilot.pause()
            message = app.screen.query_one(Message)
            self.assertTrue(message.has_class(Tone.DONE.value))
            self.assertFalse(message.has_class(Tone.PROBLEM.value))

        self.assertEqual(app.answers, Answers(Wifi('Home', 's3cret'), LYON.place, 'Europe/Paris', ['LFLL']))
        self.assertEqual(written, [app.answers.files(AIRPORTS)])

    async def test_a_wifi_the_device_cannot_keep_is_refused_on_the_spot(self):
        app, _ = setup_app(Answers())
        async with app.run_test() as pilot:
            await type_into(pilot, '#ssid', 'x' * 33)
            await type_into(pilot, '#password', '')
            self.assertIn('32 bytes', str(app.screen.query_one(Message).render()))

    async def test_a_card_gone_before_writing_is_said_and_can_be_tried_again(self):
        known = Answers(Wifi('Home', 's3cret'), LYON.place, 'Europe/Paris', ['LFLY'])

        def unplugged(files):
            raise Unreachable('No Habity found on USB.')

        app = SetupApp(known, search=lambda name: [], airports_around=lambda place: AIRPORTS, write=unplugged)
        async with app.run_test() as pilot:
            await type_into(pilot, '#password', 's3cret')
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
        app, written = setup_app(known)
        async with app.run_test() as pilot:
            await type_into(pilot, '#password', 's3cret')
            await pilot.click(Button)
            await pilot.click(Button)
            await app.workers.wait_for_complete()
            await pilot.pause()
            self.assertEqual(app.screen.query_one(SelectionList).selected, ['LFLY'])
            await pilot.press('escape')
            self.assertIsInstance(app.screen, TimeZoneScreen)
            self.assertEqual(len(app.screen.query_one(OptionList).options), 1)


if __name__ == '__main__':
    unittest.main()
