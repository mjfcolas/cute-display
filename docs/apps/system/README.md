# System app

Opens the other apps, and holds the device's settings. Its title line shows which
version of Cute Display is running.

- **Open it**: click the wheel, from any app.
- **Wheel**: move through the apps, then the settings. If there are too many apps for
  the screen, the list scrolls, and an arrow shows that there are more above or below.
- **Wheel click or long button**: open an app, or change a setting to its next value.
- **Yellow**: go back to the app that was open before.

## Settings

- **Backlight**: how long the screen stays lit after you use a button or the
  wheel.
- **Reading lamp**: off, 10, 30, 50 or 100 %.

They are kept on the SD card in `cute-display/settings.conf`. The device writes this
file. Without a card, they are kept only until the power goes off.

## Where the device is

`cute-display/general.conf` holds what all apps share. The setup writes it:

- the place that the apps show information about;
- the time zone, in the POSIX `TZ` format. Without it, the time zone is Central European
  Time.
- `time_zone_name` is only for the setup. The device ignores it.

A new time zone shows within one minute. A new place shows at the apps' next update.

```text
# cute-display/general.conf
place = Notre-Dame
latitude = 48.8530
longitude = 2.3499
time_zone = CET-1CEST,M3.5.0,M10.5.0/3
time_zone_name = Europe/Paris
```

## The apps

The `apps` line, in the same file, lists the apps that the clock runs, in the order the
system app shows them. The first one is on screen when the device starts. Without this
line, every app runs. `apps =` with nothing after it leaves only the system app. The
setup asks which apps you want, and keeps their order. The device reads this line when
it starts.

```text
# cute-display/general.conf
apps = alarm, weather, radar
```
