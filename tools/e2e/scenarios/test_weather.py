def test_the_recorded_forecast_shows_today_then_the_week(display):
    display.until_front('alarm')
    display.tap('wheel')
    display.until_front('system')
    display.turn(1)
    display.until_chosen('app Weather')
    display.tap('wheel')
    today = display.until_screen(lambda screen: screen.first_value('now') is not None, 'no forecast came')
    assert today.first_value('title') == 'Notre-Dame'
    assert today.first_value('now') == 'Partly cloudy 20°'
    assert today.first_value('day') == '17° / 21° rain 100%'
    assert today.values('hour')[0] == '14 Partly cloudy 20° 0.0 mm'

    display.turn(1)
    week = display.until_screen(lambda screen: screen.first_value('page') == 'week', 'the week did not show')
    assert week.values('day')[1] == 'Fri Cloudy 13° / 22° rain 0%'
