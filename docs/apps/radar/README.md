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
- **Airports**: `just radar-airports` fetches those within 100 km of the place (from
  OurAirports) and writes `cute-display/airports.conf`. Run it again when the place
  changes.
- `airport_labels` says which airports carry their code; the others are plain dots.
