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
| `domain`         | Every functional concept (alarm, schedule, settings, sound…) and the contracts it needs from the outside world | nothing          |
| `infrastructure` | Implementations of the domain's contracts: settings on the SD card, time from the RTC and NTP…  | `domain`, `hal`  |
| `hal`            | Contracts with the hardware: display, encoder, buttons, lights, speaker, clock, storage, radio, power | nothing          |
| `drivers`        | Implementations of `hal` on the board's chips                                                  | `hal`            |
| `ui`             | Screens: renders domain state, turns controls into domain intents. Draws on its own thread    | `domain`         |
| `firmware`       | Composition roots: builds the drivers from the pin map (`board`), wires the layers, runs them  | everything       |

The arrows are the only allowed dependencies. In particular the domain never sees a HAL
type, the UI never sees hardware, and nothing but `firmware` knows which chip is on
which pin.

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
| main     | the application loop   |                                                   |
| painter  | the e-paper display    | a refresh blocks for 0.35 to 3 s                  |
| chimes   | the speaker            | playback blocks until the sound ends              |
| survey   | the Wi-Fi radio        | a scan blocks for seconds                         |
| buttons  | the button pins        | presses must be counted while everyone else is busy |

## Where things live

```
hal/        contracts with the hardware, and the Frame the display shows
drivers/    ESP32-S3 implementations of hal; chip logic that needs no ESP32 is host-tested
hwtest/     the hardware test bench (see below)
firmware/   board pin map + one binary per image; the only crate built for the ESP32 only
tools/      host scripts (frame dump to PNG)
```

## Current state: the hardware test

The first image is a **hardware test bench** (`hwtest`), and it deliberately has no
domain, infrastructure or UI layer: it exercises the HAL directly and reports what each
part of the board says. It is kept once the application exists, as the way to check a
board.

- `hwtest::bench` — the loop: controls drive the lights, the speaker and the page;
  sensors are read every second.
- `hwtest::report` — what a check is, and how each reading becomes one.
- `hwtest::screen` — the report page and the checkerboard, drawn into a `Frame`.
- `firmware/src/bin/hwtest.rs` — builds the board, hands its devices to the bench.

`domain`, `infrastructure` and `ui` arrive with the application.
