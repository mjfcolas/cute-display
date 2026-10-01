def test_the_system_app_opens_another_app_and_goes_back_to_it(display):
    display.until_front('alarm')
    display.tap('wheel')
    display.until_front('system')
    display.until_chosen('app Alarm clock')
    display.turn(1)
    display.until_chosen('app Weather')
    display.tap('wheel')
    display.until_front('weather')

    display.tap('wheel')
    display.until_front('system')
    display.tap('yellow')
    display.until_front('weather')
