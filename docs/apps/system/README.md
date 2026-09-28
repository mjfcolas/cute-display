# System app

The app launcher and the device's settings. Its title line says which version of
Cute Display runs.

- **Open**: click the wheel, from any app.
- **Wheel**: move through the apps, then the settings.
- **Wheel click or long button**: open an app, or move a setting to its next value.
- **Yellow**: back to the app it was opened from.

## Settings

- **Backlight**: how long the screen stays lit after a touch.
- **Reading lamp**: off, 10, 30, 50 or 100 %.

They are kept on the SD card in `cute-display/settings.conf`, written by the device.
Without a card, they last until the power goes.

## Where the device is

What every app shares, in `cute-display/general.conf`, written by the setup: the place
the weather and the radar are about, and the time zone, in POSIX `TZ` form (Central
European time without it). `time_zone_name` is the setup's own; the device ignores it.
A new time zone shows within a minute; a new place at the apps' next update.

```text
# cute-display/general.conf
place = Notre-Dame
latitude = 48.8530
longitude = 2.3499
time_zone = CET-1CEST,M3.5.0,M10.5.0/3
time_zone_name = Europe/Paris
```
