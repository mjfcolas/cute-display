# Design

## Layers

The engine, which the apps run on:

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

The apps run on the engine. `catalog` lists them, and both composition roots use it:

```
   apps/<app> ──▶ libs/<lib>
        │              │
        └──────┬───────┘
               ▼
   engine: domain (what it gives to an app), ui (the shell, the toolkit), conf_text
```

`maintenance` and `hwtest` are not in the diagrams. They depend on `hal` only.

| Layer            | Holds                                                                                          | Depends on       |
| ---------------- | ---------------------------------------------------------------------------------------------- | ---------------- |
| `domain`         | The concepts that every app uses (the foreground app, settings, lighting, the time, the device's place, sound…). The contracts the engine gives to apps. The contracts it needs from the outside world. It speaks in intents (`request_refresh`, `choose_next_backlight`), never in controls ("yellow pressed") | nothing          |
| `infrastructure` | The domain's contracts, implemented on the HAL. Also HAL contracts that are built from two implementations (a button, a wheel), shared by two users (the RTC), or limited to the image's directory on the card. For the test image: HTTP and UDP sent to stubs on the local network | `domain`, `hal`, `conf_text` |
| `conf_text`      | The device's conf files, as `key = value` lines: the engine's files, and the files of any app that uses this format | nothing |
| `hal`            | Contracts with the hardware: display, encoder, buttons, lights, speaker, clock, thermometer, storage, radio, HTTP, UDP, I2C bus, power, system, and the steady clock that the app image runs on | nothing          |
| `drivers`        | Implementations of `hal` for the board's chips                                                 | `hal`            |
| `hal_testing`, `domain_testing` | For tests only: test doubles of the `hal` contracts and of the contracts that `domain` needs from the outside world. Also the checks that every implementation of a contract must pass, both the doubles and the real ones | `hal`; `domain` |
| `ui_testing`     | For tests only: the rendering references that a screen's drawing is compared with | `hal` |
| `ui`             | The shell that hosts the foreground app, the system app's screen, and the toolkit that apps draw with. It reads the controls, turns them into domain intents, and draws the domain state. It runs on its own thread | `domain`         |
| `maintenance`    | The console on the USB cable, or on a Unix socket in the simulator. It gives access to the SD card. For tests, it also gives a remote for the buttons and the wheel, shows what the screen shows and says, what the lights and the speaker do, and reads and sets the RTC | `hal`            |
| `hwtest`         | The hardware test bench                                                                        | `hal`            |
| `libs`           | Code that several apps share. Each app has its own instance of it | `domain`, `ui`, `conf_text`, other `libs` |
| `apps`           | One crate per app, with the same layers as the engine: `domain` holds its concepts and the contracts they need; `infrastructure` implements those contracts on what the engine gives; `ui` is its screen. The crate root installs the app | `domain`, `ui`, `conf_text`, `libs` |
| `app`            | The app image, on any hardware that implements the HAL contracts. It wires the layers, runs the threads, and runs the apps it is given, in the card directory it is given | `domain`, `infrastructure`, `ui`, `hal` |
| `catalog`        | The apps that an image can contain, selected by its Cargo features. The board and the computer use the same list | `apps`, `ui`, `hal` |
| `firmware`       | The composition roots on the board. Each one builds the drivers from the pin map (`board`) and gives them, with the catalog's apps, to `app`. The app image uses `cute-display/` and the Internet. The test image (`e2e`) uses `cute-display-e2e/` and stubs on the local network. The hardware test image gives the drivers to `hwtest` | everything except `simulator` |
| `simulator`      | The composition root on a computer ([its README](docs/simulator/README.md)). It implements the HAL with a window (or no window), the keyboard, a directory, and the computer's network or a recorded one, on a clock that can run faster. It gives this HAL and the catalog's apps to `app`, and serves the maintenance console on a Unix socket. The panel refreshes with the same policy as the UC8253 driver | `app`, `catalog`, `hal`, `infrastructure`, `maintenance`, `drivers` (its host half); `domain`, `ui`, `libs`, `apps` and `domain_testing` for its screen preview |

The arrows are the only allowed dependencies. In particular, the domain never sees a HAL
type, the UI never sees the hardware, `app` never sees a chip, and only `firmware` knows
which chip is on which pin. An app does not see the HAL or another app: everything it
gets from outside comes through the engine's contracts.

Tests are the only exception:

- In tests, `ui`, the apps and the libs draw on `hal`'s `Frame`.
- The tests of any crate can use `hal_testing`, `domain_testing` and `ui_testing`. All of
  them are dev-dependencies.
- The integration tests of `app` run the apps of `catalog`.
- The simulator's screen preview is an example. It uses `domain_testing` in the same way.

The UI talks to the outside through three interfaces, which `app` connects:

- **Output**: embedded-graphics' `DrawTarget`. It is a library trait. `hal::display::Frame`
  implements it, and every screen draws on it.
- **Description**: `ui::Description` is a frame described in words. When the frame is
  shown, it is given as lines to the function that the composition root passes to
  `app::run`. The composition roots give these lines to the maintenance console.
- **Input**: `ui::controls::ControlsSample` is plain data that the UI owns (detents,
  presses, buttons held). It is filled from the HAL's `RotaryEncoder` and `PushButton`.

## SOLID, applied

- **Single responsibility**: one crate per layer, one module per concept. A driver drives
  one chip. A screen shows one thing.
- **Open/closed**: new hardware is a new driver behind an existing contract. A new
  storage backend is a new infrastructure implementation. Neither one changes its
  callers.
- **Liskov**: every implementation follows the documented behaviour of its contract.
  This is why the host tests can replace the hardware with test doubles. The checks in
  `hal_testing` and `domain_testing` test each implementation against it.
- **Interface segregation**: contracts are small and named for one capability
  (`RotaryEncoder`, `PushButton`, `DimmableLight`). There is no single interface for the
  whole board.
- **Dependency inversion**: the layer that needs something owns the contract
  (`domain` for the application, `hal` for the hardware). The layer that implements it
  depends on it, never the reverse.

## Contracts

- How HAL contracts work with threads: see the doc of the `hal` crate.
- Failures are values: a `hal::Fault` describes what the hardware did not do.
- Contracts use the product's units (`Brightness`, `Temperature`, `Redraw`), not
  register values.

## Threads

A slow device gets its own thread. Other threads send it requests through a channel, and
read its latest value. The thread periods use `hal::steady`, which the simulator can run
faster than real time:

| Thread   | Owns                   | Why                                               |
| -------- | ---------------------- | ------------------------------------------------- |
| main     | the clock, lighting, each app's `tick` | they must keep running, whatever the screen shows |
| ui       | controls, apps, display | a refresh blocks for 0.35 to 7 s                 |
| buttons  | the button pins        | presses must be counted while the other threads are busy |
| speaker  | the speaker            | playing blocks until the sound ends or is stopped  |
| maintenance | the input of the USB console (of a Unix socket in the simulator) | it waits for lines from a computer |
| network  | the Wi-Fi, HTTPS and UDP clients, for the time and each app's `fetch` | a fetch waits for the network for several seconds. There is only one thread because each network stack uses internal RAM |

## Where things are

```
src/            the sources, one crate per directory
  engine/         what the apps run on
    domain/         what every app uses: the foreground app and what the engine gives
                    to an app, settings, lighting, the time and time zones, the
                    device's place, sound
    infrastructure/ the domain's contracts on the HAL: conf files, lights, the RTC, the
                    speaker, Internet on demand, NTP; controls built from two parts, a
                    shared RTC, the image's directory on the card; in `testing`, what
                    only the test image or the simulator needs
    ui/             the shell, gestures, the system app, and the toolkit that apps draw
                    with; tested on the host with hal's Frame as a dev-dependency
    ui_testing/     for tests: the rendering references for screens
    conf_text/      the `key = value` format of the device's conf files
    hal/            contracts with the hardware, and the Frame that the display shows
    hal_testing/    for tests: test doubles of hal, and the checks that each
                    implementation must pass
    domain_testing/ for tests: test doubles of what the domain needs from the outside
                    world, and the checks that each implementation must pass
    drivers/        ESP32-S3 implementations of hal; chip logic that does not need an
                    ESP32 is tested on the host; in `testing`, what only the test image
                    runs
  libs/           code that several apps share
  apps/           one crate per app
  catalog/        the apps that an image can contain, the same for the board and the
                  computer
  maintenance/    the console on the USB cable (a Unix socket in the simulator): the SD
                  card, and the remote for tests
  hwtest/         the hardware test bench
  app/            the app image's threads and wiring, generic over the HAL
  firmware/       the board's pin map, and one binary per image (`app`, `e2e`,
                  `hwtest`); the one crate that builds for the ESP32 and nothing else
  simulator/      the app image on a computer, and the screen preview
docs/           installing, troubleshooting, development, releasing, the board, and a
                README for each app, the console, the hardware test and the simulator
tools/          tools for the computer: the [installer](tools/installer/README.md)
                (the device's flash and SD card, the step-by-step setup), whole-flash
                test images for it, the [link](tools/link/README.md) to the device's
                console (shared with the tests, and able to turn a frame dump into a
                PNG), and the [end-to-end scenarios](tools/e2e/README.md) on the
                simulator and the device
```

## Applications

The main image runs small apps, one in the foreground at a time. Its code is in
`src/app/`. On the board, `src/firmware/src/app_image.rs` starts it; on a computer,
`src/simulator/` does. The engine has one app of its own, the system app. The other apps
come from `src/apps/`. `catalog` lists them, and its Cargo features select the apps that
an image contains. At start, `general.conf` chooses which of them run, and in which
order.

- **An app is a crate**, with the engine's layers. Its `domain` needs nothing else from
  the app except its `ID`. Its `infrastructure` and `ui` are built on its `domain`. Its
  root gives a `ui::Installable`: its `ID`, its title, and an `install` function. This
  function takes what the engine gives to it, `domain::apps::Services`, and returns a
  `ui::InstalledApp`. A `ui::InstalledApp` holds two things: a
  `domain::apps::AppService`, which keeps running whichever app is in the foreground,
  and the app's screen, a `ui::Screen`.
- **What the engine gives** is all that an app gets from outside: the foreground app,
  the clock, the device's place, the Internet, the files in its own directory
  (`apps/<id>/`, in the image's directory on the card), and the sound.
- **Apps are independent**: no app depends on another. Code that several apps share is
  in a `libs` crate, and each app has its own instance of it.
- **The domain decides which app is in the foreground**: `Foreground` says which one.
  Anything can bring an app to the foreground, not only the controls. For example, an
  app's `tick` can.
- **Fetching**: on the network thread, the apps take turns. Each app says if a fetch is
  due, and then fetches. Only one fetch runs at a time, so an app must keep its requests
  few and small. The engine turns the Wi-Fi on for a fetch, and off when it is idle.
- **The UI decides how apps are shown and controlled**: an app's `ui::Screen` turns the
  controls into the app's intents and gives its UI state. The UI state draws and
  describes itself (`ui::DrawWithin`, `ui::Describe`). `ui::Shell` hosts the foreground
  app. `ui::gestures` turns the controls into what the shell and the apps receive.
- **Refreshing**: the app image only asks to show its changes. The panel driver's policy
  decides when the screen gets a clean refresh. A screen uses `Screen::version` to
  change by itself.

## More

- [Simulator](docs/simulator/README.md)
- [Maintenance console](docs/maintenance/DESIGN.md)
- [Hardware test](docs/hwtest/DESIGN.md)
- [Hardware](docs/hardware.md)
