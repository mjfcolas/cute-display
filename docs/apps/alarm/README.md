# Alarm clock

The time, large, and a wake-up time for each day of the week. It is the app in front
when the device starts.

- **Yellow**: alarm on or off, the wake-up times kept.
- **Long button**: the wake-up times, starting on today.
  - **Wheel**: choose a day; **long button**: set its hour, then its minutes.
  - Turning the hour past 23 or below 0 takes the alarm off that day.
  - **Yellow**: while setting a day, puts its time back; otherwise, back to the clock.
- Half an hour before, the reading lamp and the front light rise like the sun.
- Then it rings, softly at first and louder over a minute, and comes to the front
  whatever app was there.
  - **Long button**: snooze for 9 minutes.
  - **Yellow and long button held together for a second**: stop until the next alarm,
    ringing or snoozing.
  - Unanswered, it stops after a quarter of an hour.
- A wake-up time passed while the device was off, or set after it passed, waits for its
  next day.

## The time

- Set from the Internet once a day, over the Wi-Fi of the
  [weather app](../weather/README.md); the battery-backed clock keeps it meanwhile.

## Configuration

- `cute-display/clock.conf`: the time zone, in POSIX `TZ` form; Central European time
  without it. Changes show within a minute.

```text
# cute-display/clock.conf
time_zone = EST5EDT,M3.2.0,M11.1.0
```

- `cute-display/alarm.conf`: the wake-up times, written by the device. Without a card,
  they last until the power goes.
