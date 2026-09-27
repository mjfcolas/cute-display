# Weather app

Today's weather and the week's at one place, from [Open-Meteo](https://open-meteo.com).
The [alarm clock](../alarm/README.md) shows today's and the coming hours.

- **Today**:
  - the weather now, how it feels, the day's range and chance of rain;
  - the humidity, the pressure at sea level, the wind, its arrow pointing where the air
    goes, and the sun's hours;
  - twelve hours from the one under way: their chance of rain when likely, and what
    falls as a bar.
- **The week**: each day's sky, range and chance of rain.
- **Wheel**: today ↔ the week.
- **Long button**: update now.
- Updates every hour on its own; the foot of the screen says how fresh the forecast is.

## Configuration

Two files on the SD card, put there with `just sd-put`:

```text
# cute-display/weather.conf
place = Paris
latitude = 48.85
longitude = 2.35
```

```text
# cute-display/wifi.conf, shared with every app that goes online
ssid = MyNetwork
password = secret
```

Changes are picked up at the next update, with no restart.
