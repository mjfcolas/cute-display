import unittest

from cute_display_installer.flash.layout import AppImage, Placement, Security, Slot
from cute_display_installer.setup.offer import Offering, offer
from cute_display_installer.updating import entries
from test_layout import HABITY_1_1_1, habity, slots

OLD, LATEST = AppImage('cute-display', '2026.9.0'), AppImage('cute-display', '2026.10.0')
UPDATING = entries('# Updating\n\n## 2026.10.0\n\n- The alarm is set again.\n')


class Offers(unittest.TestCase):
    def test_a_clock_without_cute_display_is_offered_it_where_it_would_go(self):
        found = offer(habity(), LATEST)
        self.assertEqual((found.offering, found.placement), (Offering.INSTALL, Placement(Slot.APP1)))

    def test_an_older_cute_display_is_offered_the_update(self):
        found = offer(habity(booting=Slot.APP1, slots=slots(HABITY_1_1_1, OLD)), LATEST)
        self.assertEqual((found.offering, found.installed, found.slot), (Offering.UPDATE, OLD, Slot.APP1))

    def test_the_update_comes_with_what_to_know_before_it(self):
        found = offer(habity(booting=Slot.APP1, slots=slots(HABITY_1_1_1, OLD)), LATEST, UPDATING)
        self.assertEqual([entry.version for entry in found.to_know], ['2026.10.0'])

    def test_nothing_to_know_when_nothing_is_updated(self):
        self.assertEqual(offer(habity(), LATEST, UPDATING).to_know, ())
        self.assertEqual(offer(habity(booting=Slot.APP0, slots=slots(HABITY_1_1_1, LATEST)), LATEST, UPDATING).to_know,
                         ())

    def test_a_snapshot_or_a_modified_build_is_always_offered_the_update(self):
        for installed, latest in ((AppImage('cute-display', '2026.10.0-snapshot'), LATEST),
                                  (AppImage('cute-display', '2026.10.0-dirty'), LATEST),
                                  (LATEST, AppImage('cute-display', '2026.10.1-snapshot'))):
            found = offer(habity(booting=Slot.APP1, slots=slots(HABITY_1_1_1, installed)), latest)
            self.assertIs(found.offering, Offering.UPDATE, (installed, latest))

    def test_an_up_to_date_cute_display_that_runs_is_left_alone(self):
        self.assertIs(offer(habity(booting=Slot.APP1, slots=slots(HABITY_1_1_1, LATEST)), LATEST).offering,
                      Offering.NOTHING)
        self.assertIs(offer(habity(booting=Slot.APP1, slots=slots(HABITY_1_1_1, LATEST)), OLD).offering,
                      Offering.NOTHING)

    def test_an_up_to_date_cute_display_that_does_not_run_is_offered_the_start(self):
        found = offer(habity(booting=Slot.APP0, slots=slots(HABITY_1_1_1, LATEST)), LATEST)
        self.assertEqual((found.offering, found.slot), (Offering.START, Slot.APP1))

    def test_an_update_the_clock_refuses_is_not_offered(self):
        refused = habity(booting=Slot.APP0, security=Security(True, False), slots=slots(HABITY_1_1_1, OLD))
        self.assertIs(offer(refused, LATEST).offering, Offering.START)


if __name__ == '__main__':
    unittest.main()
