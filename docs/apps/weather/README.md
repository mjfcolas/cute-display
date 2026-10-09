# Weather app

The weather for today and for the week at one place, from
[Open-Meteo](https://open-meteo.com). The [alarm clock](../alarm/README.md) shows today's
weather and the next hours.

- **Today**:
  - the weather now, the temperature it feels like, the day's lowest and highest
    temperatures, and the chance of rain;
  - the humidity, the pressure at sea level, the wind (its arrow points where the air
    goes), and the sunrise and sunset times;
  - twelve hours, starting with the current hour: their chance of rain when rain is
    likely, and the amount of rain or snow as a bar.
- **The week**: for each day, the sky, the lowest and highest temperatures, and the
  chance of rain.
- **Wheel**: switch between today and the week.
- **Long button**: update now.
- It updates by itself every hour. The bottom of the screen says how old the forecast is.

## Configuration

- **Place**: the device's place, in `cute-display/general.conf`. See the
  [system app](../system/README.md#where-the-device-is).
- **Wi-Fi**: `cute-display/wifi.conf`, shared by every app that uses the Internet.

```text
# cute-display/wifi.conf
ssid = MyNetwork
password = secret
```

Changes are used at the next update, with no restart. A long press updates now.
