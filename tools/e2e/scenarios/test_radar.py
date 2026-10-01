def test_the_recorded_aircraft_are_on_the_scope_the_nearest_listed(display):
    display.until_front('alarm')
    display.tap('wheel')
    display.until_front('system')
    display.turn(2)
    display.until_chosen('app Radar')
    display.tap('wheel')
    radar = display.until_screen(lambda screen: screen.values('aircraft'), 'no aircraft came')
    assert radar.first_value('range') == '25 km'
    assert len(radar.values('aircraft')) == 12
    assert radar.values('nearest')[0] == 'WL QAF9 3775 ft 9.3 km'
    assert 'LFPO 15 km S' in radar.values('airport')
