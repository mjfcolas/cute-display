# Design

## Layers

The engine, what the apps run on:

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

The apps, on it, picked by the composition roots:

```
   apps/alarm, apps/weather ──▶ libs/forecast        apps/radar
            │                        │                   │
            └────────────────────────┼───────────────────┘
                                     ▼
     engine: domain (what it lends an app), ui (the shell, the toolkit), conf_text
```

`maintenance` and `hwtest`, beside the diagrams, depend on `hal` alone.

| Layer            | Holds                                                                                          | Depends on       |
| ---------------- | ---------------------------------------------------------------------------------------------- | ---------------- |
| `domain`         | The concepts every app relies on (the app in front, settings, lighting, the time, where the device is, sound…), the contracts the engine lends apps, and the contracts it needs from the outside world. Speaks in intents (`request_refresh`, `choose_next_backlight`), never in controls ("yellow pressed") | nothing          |
| `infrastructure` | Implementations of the domain's contracts on the HAL | `domain`, `hal`, `conf_text` |
| `conf_text`      | The device's conf files as `key = value` lines: the engine's own, and any app's that takes up the format | nothing |
| `hal`            | Contracts with the hardware: display, encoder, buttons, lights, speaker, clock, thermometer, storage, radio, HTTP, UDP, I2C bus, power, system | nothing          |
| `drivers`        | Implementations of `hal` on the board's chips                                                  | `hal`            |
| `ui`             | The shell that hosts the app in front, the system app's screen, and the toolkit apps draw with: reads the controls, turns them into domain intents, renders domain state. Runs on its own thread | `domain`         |
| `maintenance`    | The console that serves the SD card on the USB cable                                           | `hal`            |
| `hwtest`         | The hardware test bench                                                                        | `hal`            |
| `libs`           | What several apps share, as code: each app has its own instance of it | `domain`, `ui`, `conf_text`, other `libs` |
| `apps`           | One crate per app, layered as the engine is: `domain`, its concepts and the contracts they need; `infrastructure`, those contracts on what the engine lends; `ui`, its screen. Its root installs it | `domain`, `ui`, `conf_text`, `libs` |
| `app`            | The app image on any hardware that keeps the HAL's contracts: wires the layers, runs the threads, runs the apps it is given | `domain`, `infrastructure`, `ui`, `hal` |
| `firmware`       | Composition roots on the board: builds the drivers from the pin map (`board`), hands them and the apps to `app`, or hands them to `hwtest` | everything but `simulator` |
| `simulator`      | Composition root on a computer: the HAL in a window, on the keyboard, in a directory, on the computer's Internet; hands it and the apps to `app`; the panel refreshes by the UC8253 driver's policy | `app`, `apps`, `ui`, `hal`, `infrastructure`, `drivers` (its host half); `domain` and `libs` for its screen preview |

The arrows are the only allowed dependencies. In particular the domain never sees a HAL
type, the UI never sees hardware, `app` never sees a chip, and nothing but `firmware`
knows which chip is on which pin. An app sees neither the HAL nor another app: what it
gets from outside comes through the engine's contracts. Tests are the one exception:
`ui`, the apps and the libs draw on `hal`'s `Frame` there, as a dev-dependency.

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
| main     | the clock, lighting, each app's `tick` | they must keep running whatever is on screen |
| ui       | controls, apps, display | a refresh blocks for 0.35 to 7 s                 |
| buttons  | the button pins        | presses must be counted while everyone else is busy |
| speaker  | the speaker            | playing blocks until the sound ends or is stopped  |
| maintenance | the USB console's input | it waits for lines from a computer            |
| network  | the Wi-Fi, HTTPS and UDP clients, for the time and each app's `fetch` | a fetch waits on the network for seconds; one thread, since each stack is internal RAM |

## Where things live

```
src/            the sources, one crate per directory
  engine/         what the apps run on
    domain/         what every app relies on: the app in front and what the engine lends
                    an app, settings, lighting, the time and time zones, where the
                    device is, sound
    infrastructure/ the domain's contracts on the HAL: conf files, lights, the RTC, the
                    speaker, Internet on demand, NTP
    ui/             the shell, gestures, the system app, and the toolkit apps draw
                    with; host-tested with hal's Frame as a dev-dependency
    conf_text/      the `key = value` format of the device's conf files
    hal/            contracts with the hardware, and the Frame the display shows
    drivers/        ESP32-S3 implementations of hal; chip logic that needs no ESP32 is
                    host-tested
  libs/           what apps share: forecasts
  apps/           one crate per app: the alarm clock, the weather, the radar
  maintenance/    the console that serves the SD card on the USB cable
  hwtest/         the hardware test bench
  app/            the app image's threads and wiring, generic over the HAL
  firmware/       board pin map + one binary per image (`app`, `hwtest`); the only crate
                  built for the ESP32 only
  simulator/      the app image on a computer, and the screen preview
docs/           installing and troubleshooting, development and releasing, the board, and a
                README per app, console and test image
tools/          host tools: the [installer](tools/installer/README.md) (the device's flash
                and SD card, the step-by-step setup, radar airports, RTC registers), whole-flash
                test images for it, frame dump to PNG
```

## Applications

The main image (`src/app/`, on the board by `src/firmware/src/bin/app.rs`, on a computer
by `src/simulator/`) runs small apps, one in front at a time. The engine has one app of
its own, the system app; the others come from `src/apps/`. The composition roots pick
those an image holds with Cargo features; `general.conf` chooses, at start, those that
run and their order.

- **An app is a crate**, in the engine's layers: its `domain` needs nothing else of the
  app but its `ID`, its `infrastructure` and `ui` build on its `domain`. Its root has
  the `ID` and an `install` that takes what the engine lends it,
  `domain::apps::Services`, and gives a `ui::InstalledApp`: a `domain::apps::AppService`
  that runs whatever is on screen, and a `ui::AppScreen`.
- **What the engine lends** is all an app gets from outside: the app in front, the
  clock, where the device is, the Internet, the files in its own directory
  (`cute-display/apps/<id>/`), the sound.
- **Apps are independent**: none depends on another. What several share is a `libs`
  crate, and each has its own instance of it.
- **Which app is in front is the domain's**: `Foreground` says which. Anything may
  bring an app forward, not only the controls: an app's `tick` may (the alarm does).
- **Fetching**: on the network thread, one app after another, each says whether a fetch
  is due and then fetches. Only one fetch runs at a time, so an app keeps its requests
  bounded and rare; the engine brings the Wi-Fi up for a fetch and down once idle.
- **The UI owns how apps are seen and steered**: an app's `AppScreen` turns the
  controls into its intents and draws its state. `ui::Shell` hosts the one in front;
  `ui::gestures` turns the controls into what the shell and the apps receive.
- **Refreshing**: the app image only asks for its changes; the panel driver's policy
  decides when the glass gets a clean refresh. `AppScreen::version` is how a screen
  changes on its own.

## Further

- [Simulator](docs/simulator/README.md)
- [Maintenance console](docs/maintenance/DESIGN.md)
- [Hardware test](docs/hwtest/DESIGN.md)
- [Hardware](docs/hardware.md)
