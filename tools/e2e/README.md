# End-to-end scenarios

What a person does with the clock, played on the simulator through its
[console](../../docs/maintenance/README.md) alone: the controls, what the glass says, the
lights, the speaker, the clock and the card. Where they stand among the tests:
[the pyramid](../../docs/testing/README.md).

- `just e2e`: every scenario, each on a simulator of its own; `just e2e -k alarm` for
  some.
- `just e2e --window`: each simulator in its window, to watch what a scenario does; the
  keyboard and the mouse are best left alone meanwhile.
- A scenario fails on what the glass said last, and the simulator's log comes with it; a
  panic in the log fails it too.
- `cards/standard/`: the card each starts on, a copy of it.
- `web/`: the answers of Open-Meteo and adsb.fi, recorded at `AT_THE_RECORDING` in
  `scenarios/conftest.py`, the time a scenario starts at unless it gives its own.

## Recording the web again

The forecast and the aircraft must be about the time the scenarios start at. On a copy
of the card, which the simulator writes to:

```sh
cp -r tools/e2e/cards/standard /tmp/e2e-card
just sim /tmp/e2e-card --time $(date +%s) --record tools/e2e/web
```

- Open the weather and the radar, then quit.
- `AT_THE_RECORDING` becomes that time, and the weather and radar scenarios expect the
  lines the glass now says.
