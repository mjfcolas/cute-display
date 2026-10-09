# Radar app

Shows the aircraft around the device, from the open data of [adsb.fi](https://adsb.fi).

- North is up. The device's place is a cross. Each aircraft is a triangle that points in
  the direction it flies.
- Each aircraft is named by the last two letters of its registration (`F-GKXA` → `XA`).
- The five nearest aircraft are listed next to the radar, with their altitude and
  distance.
- **Wheel**: the range, from 5 to 100 km.
- **Long button**: update now.
- It updates every 15 s, only while it is on screen.

## Configuration

- **Place**: the device's place, in `cute-display/general.conf`. See the
  [system app](../system/README.md#where-the-device-is).
- It also needs `cute-display/wifi.conf`. See the [weather app](../weather/README.md).
- **Airports**: `cute-display/apps/radar/airports.conf`, one airport per line: its
  code, latitude, longitude and name. The radar draws the airports within its range.
- `cute-display/apps/radar/radar.conf`: the airports that show their code. The others
  are plain dots.

```text
# cute-display/apps/radar/radar.conf
airport_labels = LFPG, LFPO, LFPB
```

- The airports and the labels are read at start, and when the place changes.
