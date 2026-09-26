# Design

## Layers

```
          ┌──────────┐   ┌───────────┐
          │ firmware │   │ simulator │  composition roots: the board, a computer
          └────┬─────┘   └─────┬─────┘
               └──────┬────────┘
                 ┌────▼─────┐
                 │   app    │  the app image on any hardware
                 └────┬─────┘
       ┌──────────────┼──────────────────┬──────────────┐
       ▼              ▼                  ▼              ▼
   ┌──────┐   ┌────────────────┐    ┌─────────┐    ┌────────┐
   │  ui  │──▶│     domain     │◀───│ infra-  │───▶│  hal   │◀───┐
   └──────┘   │ concepts +     │    │structure│    │contracts│   │
              │ contracts      │    └─────────┘    └────────┘    │
              └────────────────┘                          ┌──────┴──┐
                                                          │ drivers │
                                                          └─────────┘
```

`maintenance` and `hwtest`, beside the diagram, depend on `hal` alone.

| Layer            | Holds                                                                                          | Depends on       |
| ---------------- | ---------------------------------------------------------------------------------------------- | ---------------- |
| `domain`         | Every functional concept (apps, settings, lighting, weather, radar, places…) and the contracts it needs from the outside world. Speaks in intents (`request_refresh`, `choose_next_backlight`), never in controls ("yellow pressed") | nothing          |
| `infrastructure` | Implementations of the domain's contracts: settings, places and airports on the SD card, lights, the Internet on demand, Open-Meteo, adsb.fi | `domain`, `hal`  |
| `hal`            | Contracts with the hardware: display, encoder, buttons, lights, speaker, clock, thermometer, storage, radio, HTTP, I2C bus, power, system | nothing          |
| `drivers`        | Implementations of `hal` on the board's chips                                                  | `hal`            |
| `ui`             | A screen per app, the system app's included: reads the controls, turns them into domain intents, renders domain state. Runs on its own thread | `domain`         |
| `maintenance`    | The console that serves the SD card on the USB cable                                           | `hal`            |
| `hwtest`         | The hardware test bench                                                                        | `hal`            |
| `app`            | The app image on any hardware that keeps the HAL's contracts: wires the layers, runs the threads | `domain`, `infrastructure`, `ui`, `hal` |
| `firmware`       | Composition roots on the board: builds the drivers from the pin map (`board`), hands them to `app` or `hwtest` | everything but `simulator` |
| `simulator`      | Composition root on a computer: the HAL in a window, on the keyboard, in a directory, on the computer's Internet; hands it to `app`; the panel refreshes by the UC8253 driver's policy | `app`, `hal`, `infrastructure`, `drivers` (its host half) |

The arrows are the only allowed dependencies. In particular the domain never sees a HAL
type, the UI never sees hardware, `app` never sees a chip, and nothing but `firmware`
knows which chip is on which pin.

The UI meets the hardware through two exchange surfaces, which `app` connects:

- **out**: embedded-graphics' `DrawTarget`, a library trait that `hal::display::Frame`
  implements and that every screen draws on;
- **in**: `ui::controls::ControlsSample`, plain data owned by the UI (detents, presses,
  buttons held), filled from the HAL's `RotaryEncoder` and `PushButton`.

## SOLID, applied

- **Single responsibility**: a crate per layer, a module per concept. A driver drives
  one chip; a screen shows one thing.
- **Open/closed**: new hardware is a new driver behind an existing contract; a new
  storage backend is a new infrastructure implementation. Neither changes its callers.
- **Liskov**: every implementation honours its contract's documented behaviour, which is
  what lets the host tests replace hardware with fakes.
- **Interface segregation**: contracts are small and named for one capability
  (`RotaryEncoder`, `PushButton`, `DimmableLight`), not one interface per board.
- **Dependency inversion**: the layer that needs something owns the contract
  (`domain` for the application, `hal` for the hardware); the implementing layer
  depends on it, never the reverse.

## Contracts

- How HAL contracts meet threads: the `hal` crate's doc.
- Failures are values: `hal::Fault` describes what the hardware did not do.
- Contracts speak in the product's units (`Brightness`, `Temperature`, `Redraw`), not
  in register values.

## Threads

A slow device gets a thread of its own, and talks to the rest through a channel in and
a "latest value" out:

| Thread   | Owns                   | Why                                               |
| -------- | ---------------------- | ------------------------------------------------- |
| main     | domain services (lighting) | they must keep running whatever is on screen  |
| ui       | controls, apps, display | a refresh blocks for 0.35 to 7 s                 |
| buttons  | the button pins        | presses must be counted while everyone else is busy |
| maintenance | the USB console's input | it waits for lines from a computer            |
| network  | the Wi-Fi and HTTPS client, for weather and radar | a fetch waits on the network for seconds; one thread, since each stack is heap TLS needs |

## Where things live

```
src/            the sources, one crate per directory
  domain/         what the product does: the apps, the one in front, settings, lighting,
                  weather, radar, places
  infrastructure/ the domain's contracts on the HAL: conf files, lights, Internet on
                  demand, Open-Meteo, adsb.fi, airports
  ui/             app screens (the system app's among them), gestures; host-tested with
                  hal's Frame as a dev-dependency
  hal/            contracts with the hardware, and the Frame the display shows
  drivers/        ESP32-S3 implementations of hal; chip logic that needs no ESP32 is
                  host-tested
  maintenance/    the console that serves the SD card on the USB cable
  hwtest/         the hardware test bench
  app/            the app image's threads and wiring, generic over the HAL
  firmware/       board pin map + one binary per image (`app`, `hwtest`); the only crate
                  built for the ESP32 only
  simulator/      the app image on a computer
docs/           the board, and a README per app, console and test image
tools/          host scripts (frame dump to PNG, SD card over USB, radar airports, RTC
                registers backup)
```

## Applications

The main image (`src/app/`, on the board by `src/firmware/src/bin/app.rs`, on a computer
by `src/simulator/`) runs small apps, one in front at a time.

- **The domain owns the apps**: `domain::apps::App` lists them, `Foreground` says which
  one is in front. Anything may bring an app forward, not only the controls (an alarm
  will).
- **The UI owns how they are seen and steered**: one `ui::AppScreen` per app turns the
  controls into domain intents and draws the domain's state. `ui::Shell` hosts the one
  in front; `ui::gestures` keeps the long-button hold for the system.
- **Refreshing**: `ui::ScreenChange` says which redraw the glass gets; `AppScreen::version`
  is how a screen changes on its own.

## Further

- [Simulator](docs/simulator/README.md)
- [Maintenance console](docs/maintenance/DESIGN.md)
- [Hardware test](docs/hwtest/DESIGN.md)
- [Hardware](docs/hardware.md)
