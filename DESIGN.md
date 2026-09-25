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
| `ui`             | The apps and the switcher between them: reads the controls, turns them into domain intents, renders domain state. Runs on its own thread | `domain`         |
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
| main     | domain services        | they must keep running whatever is on screen      |
| ui       | controls, apps, display | a refresh blocks for 0.35 to 3 s                 |
| painter  | the e-paper display (hwtest) | a refresh blocks for 0.35 to 3 s            |
| chimes   | the speaker            | playback blocks until the sound ends              |
| survey   | the Wi-Fi radio        | a scan blocks for seconds                         |
| buttons  | the button pins        | presses must be counted while everyone else is busy |

## Where things live

```
domain/     what the product does: the apps, the one in front, Counter, Ping (an example)
ui/         app screens, the switcher, gestures; host-tested with hal's Frame as a dev-dependency
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

**The UI owns how they are seen and steered.**

- `ui::AppScreen`: one per app. Turns the controls into intents on the domain
  (`counter.add`, `ping.trigger`) and draws what the domain says. `Echo` is the exception:
  a diagnostic about the controls themselves, with nothing to say to the domain.
- `ui::gestures`: holding the long button for a second is the system's, and opens the
  switcher. A shorter press of the long button reaches the app, on release; every other
  control reaches it at once.
- `ui::switcher`: the list of apps, open over the app in front. The wheel moves its dot,
  a wheel press brings the chosen app to the front of the domain, the long button closes
  it. Where the dot stands is screen state: nothing changes in the domain until a choice
  is made.
- `ui::Shell`: shows the screen of the app in front, under its title, or the switcher.

**Refreshing**: the UI thread redraws after any input, and whenever the app in front is
not the one on the glass. A new screen in front is a whole, clean redraw; anything else
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
