# Weather app

Today's weather and the week's at one place, from [Open-Meteo](https://open-meteo.com).

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
