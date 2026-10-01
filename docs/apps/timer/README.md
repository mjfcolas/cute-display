# Timer

A countdown, pomodoro rounds, or a nap, counted down in minutes, in seconds over the last
one.

- **Long button**: the mode: timer, pomodoro or nap, each keeping its own length.
- **Wheel**: the minutes, one by one up to 10, by 5 up to an hour, by the quarter after.
- **Yellow**: start.
- At the end of each count, and of each pomodoro stretch, the timer comes to the front
  whatever app was there.

## Timer

- From 1 minute to 2 hours; once over, it rings, louder and louder, until **yellow** or
  the **long button**, or for 10 minutes.
- **Yellow**: pause, and go on; **long button**: stop.

## Pomodoro

- Work for its minutes, from 5 to 60, then 5 minutes of rest, four rounds, then a
  15-minute break, a short chime between each.
- **Yellow**: pause, and go on; **long button**: stop.

## Nap

- From 5 to 90 minutes, the lights out; over its last three minutes the lights rise, then
  it rings, louder and louder, until **yellow** or the **long button**, or for 10 minutes.
- While napping, only **yellow** does anything: it stops, so that nothing half asleep
  changes a setting.

## Configuration

- `cute-display/apps/timer/timer.conf`: the mode and the length of each, written by the
  device. Without a card, they last until the power goes.
- A timer under way does not survive a restart.
