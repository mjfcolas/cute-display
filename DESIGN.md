# Design

## Layers

```
                 ┌──────────┐
                 │ firmware │  composition roots: one binary per image
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

| Layer            | Holds                                                                                          | Depends on       |
| ---------------- | ---------------------------------------------------------------------------------------------- | ---------------- |
| `domain`         | Every functional concept (alarm, schedule, settings, sound…) and the contracts it needs from the outside world. Speaks in intents ("snooze"), never in controls ("yellow pressed") | nothing          |
| `infrastructure` | Implementations of the domain's contracts: settings on the SD card, time from the RTC and NTP…  | `domain`, `hal`  |
| `hal`            | Contracts with the hardware: display, encoder, buttons, lights, speaker, clock, storage, radio, power | nothing          |
| `drivers`        | Implementations of `hal` on the board's chips                                                  | `hal`            |
| `ui`             | A screen per app, the system app's included: reads the controls, turns them into domain intents, renders domain state. Runs on its own thread | `domain`         |
| `firmware`       | Composition roots: builds the drivers from the pin map (`board`), wires the layers, runs them  | everything       |

The arrows are the only allowed dependencies. In particular the domain never sees a HAL
type, the UI never sees hardware, and nothing but `firmware` knows which chip is on
which pin.

The UI meets the hardware through two exchange surfaces, which `firmware` connects:

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

- HAL contracts are **blocking and single-owner**. Deciding what runs on which thread
  belongs to the layer that uses them.
- Failures are values: `hal::Fault` describes what the hardware did not do.
- Contracts speak in the product's units (`Brightness`, `Temperature`, `Redraw`), not
  in register values.

## Threads

A slow device gets a thread of its own, and talks to the rest through a channel in and
a "latest value" out:

| Thread   | Owns                   | Why                                               |
| -------- | ---------------------- | ------------------------------------------------- |
| main     | domain services (lighting) | they must keep running whatever is on screen  |
| ui       | controls, apps, display | a refresh blocks for 0.35 to 3 s                 |
| painter  | the e-paper display (hwtest) | a refresh blocks for 0.35 to 3 s            |
| chimes   | the speaker            | playback blocks until the sound ends              |
| survey   | the Wi-Fi radio        | a scan blocks for seconds                         |
| buttons  | the button pins        | presses must be counted while everyone else is busy |

## Where things live

```
domain/     what the product does: the apps, the one in front, settings, lighting, Counter, Ping
infrastructure/ the domain's contracts on the HAL: the settings file, the lights
ui/         app screens (the system app's among them), gestures; host-tested with hal's
            Frame as a dev-dependency
hal/        contracts with the hardware, and the Frame the display shows
drivers/    ESP32-S3 implementations of hal; chip logic that needs no ESP32 is host-tested
hwtest/     the hardware test bench (see below)
firmware/   board pin map + one binary per image (`app`, `hwtest`); the only crate built
            for the ESP32 only
tools/      host scripts (frame dump to PNG)
```

## Applications

The main image (`firmware/src/bin/app.rs`) is a set of small apps, one in front at a
time.

**The domain owns the apps.** `domain::apps::App` lists them and `Foreground` says which
one is in front; anything may bring an app forward, not only the person at the controls
(when an alarm rings, the alarm's own service will). Each app's state and rules are domain
concepts too (`Counter`, `Ping`), shared with whoever needs them.

**The system app is an app with a special status.** It lists the other apps and holds the
device's settings. It is not in the list (`App::LAUNCHABLE`), only the system gesture
reaches it, and leaving it goes back to the app it was opened from, which `Foreground`
remembers. Its settings are domain concepts:

- `domain::settings`: the backlight duration and the reading lamp's level (off, 10, 30,
  50, 100 %), kept by
  a `SettingsStore` on every change. `infrastructure::settings_file` keeps them as a few
  lines of text, `cute-display.conf` at the root of the SD card; without a card they last
  until the power goes.
- `domain::lighting`: the light over the screen comes up when the controls are touched
  and goes out once the backlight duration has passed; the reading lamp shines at its
  setting. The domain decides every level, the screen's 20 % included; lights are a
  `Light` contract taking a level, which `infrastructure::hal_light` implements on the
  HAL's `DimmableLight`. The main thread refreshes it ten times a second.

**The UI owns how they are seen and steered.**

- `ui::AppScreen`: one per app, the system app's included. Turns the controls into
  intents on the domain and draws what the domain says. `Echo` is the exception: a
  diagnostic about the controls themselves, with nothing to say to the domain.
- `ui::apps::SystemScreen`: one list, the apps then the settings. The wheel moves the dot,
  which starts on the app the system app was opened from; a press opens an app or moves a
  setting to its next value; the long button goes back.
- `ui::gestures`: holding the long button for a second is the system's: it opens the
  system app, or closes it. A shorter press of the long button reaches the app, on
  release; every other control reaches it at once. Any control used also tells
  `Lighting` that the device was touched.
- `ui::Shell`: shows the screen of the app in front under its title, and tells a screen
  when it comes to the front.

**Refreshing**: the UI thread redraws after any input, and whenever the app in front is
not the one on the glass. A new app in front is a whole, clean redraw; anything else
only redraws what changed. Controls keep counting during a refresh (PCNT, the buttons'
thread), and what piled up is handled in one go before the next refresh. A change in the
domain that nobody made through the controls, inside the app in front, is not noticed
yet: the first app that needs it decides how.

## The hardware test

A second image, the **hardware test bench** (`hwtest`), exercises the HAL directly and
reports what each part of the board says; it has no domain or UI layer on purpose. It is
the way to check a board.

- `hwtest::bench` — the loop: controls drive the lights, the speaker and the page;
  sensors are read every second.
- `hwtest::report` — what a check is, and how each reading becomes one.
- `hwtest::screen` — the report page and the checkerboard, drawn into a `Frame`.
- `firmware/src/bin/hwtest.rs` — builds the board, hands its devices to the bench.
