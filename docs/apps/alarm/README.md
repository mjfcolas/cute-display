# Alarm clock

The time, large, today's weather, and a wake-up time for each day of the week. First
among the [apps](../system/README.md#the-apps) unless they say otherwise, it is in front
when the device starts.

- **Yellow**: alarm on or off, the wake-up times kept.
- **Wheel**: the weather's later hours, and back.
- **Long button**: the settings, a wake-up time per day and the ringtone, starting on
  today.
  - **Wheel**: choose a day; **long button**: set its hour, then its minutes.
  - Turning the hour past 23 or below 0 takes the alarm off that day.
  - **Wheel** past Sunday: the ringtone; **long button**, then the **wheel** goes round
    the ringtones, each playing softly; **long button**: done.
  - **Yellow**: while setting a day or the ringtone, puts it back; otherwise, back to the
    clock.
- Half an hour before, the reading lamp and the front light rise like the sun.
- Then it rings, softly at first and louder over a minute, and comes to the front
  whatever app was there.
  - A ringtone it cannot play rings as the chime.
  - **Long button**: snooze for 9 minutes.
  - **Yellow and long button held together for a second**: stop until the next alarm,
    ringing or snoozing.
  - Unanswered, it stops after a quarter of an hour.
- A wake-up time passed while the device was off, or set after it passed, waits for its
  next day.

## The weather

- Beside the date, the day's sky and its low and high; beside the time, seven hours from
  the one under way. Coming back to the clock goes back to the hour under way.
- From Open-Meteo every hour, at the device's place (see the
  [system app](../system/README.md#where-the-device-is)); nothing shows until a forecast
  came.

## The time

- Set from the Internet once a day, over the Wi-Fi in `cute-display/wifi.conf` (see the
  [weather app](../weather/README.md)); the battery-backed clock keeps it meanwhile.
- In the device's time zone (see the [system app](../system/README.md#where-the-device-is)).

## Ringtones

- The chime, always there, and the MP3 files in `cute-display/apps/alarm/ringtones/`,
  named by their file names without `.mp3`.
- The [setup](../../install.md#run-the-setup) offers to copy Habity's own there.

## Configuration

- `cute-display/apps/alarm/alarm.conf`: the wake-up times and the ringtone, written by the
  device. Without a card, they last until the power goes.
