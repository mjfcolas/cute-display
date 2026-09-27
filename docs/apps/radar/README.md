# Radar app

The aircraft around a place, from [adsb.fi](https://adsb.fi)'s open data.

- North up, the place as a cross, a triangle per aircraft pointing where it flies.
- Aircraft are named by the last two letters of their registration (`F-GKXA` → `XA`).
- The five nearest are listed beside the scope, with altitude and distance.
- **Wheel**: range, 5 to 100 km.
- **Long button**: update now.
- Updates every 15 s, only while it is on screen.

## Configuration

```text
# cute-display/radar.conf
place = Notre-Dame
latitude = 48.8530
longitude = 2.3499
airport_labels = LFPG, LFPO, LFPB
```

- Also needs `cute-display/wifi.conf`, see the [weather app](../weather/README.md).
- **Airports**: `cute-display/airports.conf`, one per line: its code, latitude,
  longitude and name. The radar draws those within its range.
- `airport_labels` says which airports carry their code; the others are plain dots.
- Airports and labels are read at start and when the place changes.
