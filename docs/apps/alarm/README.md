# Alarm clock

Shows the time in large digits and today's weather, and has a wake-up time for each day
of the week. It is the first of the [apps](../system/README.md#the-apps), unless you
choose another order, so it is on screen when the device starts.

- **Yellow**: turns the alarm on or off. The wake-up times are kept.
- **Wheel**: shows the weather of later hours. Turn it back to return.
- **Long button**: opens the settings, a wake-up time for each day and the ringtone. It
  starts on today.
  - **Wheel**: choose a day. **Long button**: set its hour, then its minutes.
  - Turning the hour above 23 or below 0 turns the alarm off for that day.
  - **Wheel** after Sunday: the ringtone. Press the **long button**, then turn the
    **wheel** to go through the ringtones. Each one plays softly. **Long button**: done.
  - **Yellow**: while you set a day or the ringtone, it cancels the change. Otherwise, it
    goes back to the clock.
- Half an hour before the wake-up time, the reading lamp and the front light rise like
  the sun.
- Then the alarm rings, softly at first and louder over one minute. It comes on screen,
  whatever app was there.
  - If it cannot play the ringtone, it plays the chime.
  - **Long button**: snooze for 9 minutes.
  - **Hold yellow and the long button together for one second**: stop until the next
    alarm, while it rings or snoozes.
  - If nobody stops it, it stops after 15 minutes.
- If a wake-up time passes while the device is off, or if you set it after it has
  passed, the alarm waits for the next day.

## The weather

- Next to the date: the day's sky, and its lowest and highest temperatures. Next to the
  time: seven hours, starting with the current hour. When you come back to the clock, it
  shows the current hour again.
- It comes from Open-Meteo every hour, for the device's place (see the
  [system app](../system/README.md#where-the-device-is)). Nothing shows until the first
  forecast arrives.

## The time

- It is set from the Internet once a day, over the Wi-Fi in `cute-display/wifi.conf`
  (see the [weather app](../weather/README.md)). Between two updates, the battery-backed
  clock keeps it.
- It is in the device's time zone (see the
  [system app](../system/README.md#where-the-device-is)).

## Ringtones

- The chime, which is always there, and the MP3 and WAV files in
  `cute-display/apps/alarm/ringtones/`. Each one is named by its file name without the
  extension.
- The [setup](../../install.md#run-the-setup) offers to copy Habity's own ringtones there.

## Configuration

- `cute-display/apps/alarm/alarm.conf`: the wake-up times and the ringtone. The device
  writes it. Without a card, they are kept only until the power goes off.
