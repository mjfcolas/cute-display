# Timer

A countdown, pomodoro rounds, or a nap. It counts down in minutes, and in seconds during
the last minute.

- **Long button**: the mode: timer, pomodoro or nap. Each mode keeps its own length.
- **Wheel**: the minutes, one at a time up to 10, then 5 at a time up to one hour, then
  15 at a time.
- **Yellow**: start.
- At the end of each countdown, and of each pomodoro period, the timer comes on screen,
  whatever app was there.

## Timer

- From 1 minute to 2 hours. At the end, it rings louder and louder until you press
  **yellow** or the **long button**, or for 10 minutes.
- **Yellow**: pause, and continue. **Long button**: stop.

## Pomodoro

- Work for the chosen time, from 5 to 60 minutes, then rest for 5 minutes. After four
  rounds, a 15-minute break. A short chime plays between periods.
- **Yellow**: pause, and continue. **Long button**: stop.

## Nap

- From 5 to 90 minutes, with the lights off. During the last three minutes, the lights
  rise. Then it rings louder and louder until you press **yellow** or the **long
  button**, or for 10 minutes.
- During a nap, only **yellow** works, and it stops the nap. So you cannot change a
  setting by mistake while you are half asleep.

## Configuration

- `cute-display/apps/timer/timer.conf`: the mode and the length of each mode. The device
  writes it. Without a card, they are kept only until the power goes off.
- A running timer is lost when the device restarts.
