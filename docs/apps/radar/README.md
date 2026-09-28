# Radar app

The aircraft around the device, from [adsb.fi](https://adsb.fi)'s open data.

- North up, the place as a cross, a triangle per aircraft pointing where it flies.
- Aircraft are named by the last two letters of their registration (`F-GKXA` → `XA`).
- The five nearest are listed beside the scope, with altitude and distance.
- **Wheel**: range, 5 to 100 km.
- **Long button**: update now.
- Updates every 15 s, only while it is on screen.

## Configuration

- **Where**: the device's place, in `cute-display/general.conf`, see the
  [system app](../system/README.md#where-the-device-is).
- Also needs `cute-display/wifi.conf`, see the [weather app](../weather/README.md).
- **Airports**: `cute-display/apps/radar/airports.conf`, one per line: its code, latitude,
  longitude and name. The radar draws those within its range.
- `cute-display/apps/radar/radar.conf`: which airports carry their code; the others are
  plain dots.

```text
# cute-display/apps/radar/radar.conf
airport_labels = LFPG, LFPO, LFPB
```

- Airports and labels are read at start and when the place changes.
