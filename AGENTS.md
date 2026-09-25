# AGENTS.md

Firmware for the Habity bedside clock (ESP32-S3, e-paper, wheel, buttons, RTC, SD,
speaker, lights). Read [`DESIGN.md`](DESIGN.md) for the architecture and
[`docs/hardware.md`](docs/hardware.md) for what the board is.

## Rules

### 1. Naming, semantics and architecture carry the meaning

- Names say what a thing *is* or *does* in the product's or the hardware's terms:
  `Brightness::adjusted_by`, `PushButton::take_presses`, `Uc8253`, not `set_val`,
  `handle`, `Manager`, `utils`.
- Types carry invariants instead of comments: a `Brightness` cannot exceed 100 %, a
  `Redraw` says what the caller wants instead of a boolean.
- The module and crate structure mirrors the functional structure. Someone reading the
  tree should understand what the product does and where each concept lives.
- **Comment only what the semantics failed to convey**: a measured hardware behaviour,
  a non-obvious constraint, the reason something surprising is correct. Never restate
  the code, never narrate history ("used to", "was changed because"), never leave
  commented-out code. If a comment is needed to explain *what* code does, rename or
  restructure first.

### 2. Layered architecture, SOLID

Described in [`DESIGN.md`](DESIGN.md). In short: `domain` ← `infrastructure` → `hal` ←
`drivers`, `ui` → `domain`, and one composition root per image in `firmware`.
Dependencies only point the way DESIGN.md says; a new dependency that points elsewhere
is a design change and goes through DESIGN.md first.

### 3. Every piece of logic is tested on the host

- Only `drivers` (its ESP32 half) and `firmware` need the device. Everything else builds
  and tests with a stock toolchain: `just test`.
- Logic inside a driver that does not touch a peripheral (a memory layout, a register
  decoder, a debouncer) is written so it compiles on the host and is tested there.
- Code that consumes a HAL contract is tested against fakes of that contract.

### 4. No panics on the device

The panic family is linted (`unwrap_used`, `indexing_slicing`, …; tests are exempt).
Hardware failures are `hal::Fault` values that end up on the screen or in the log, and
the device keeps running.

### 5. The device is not ours alone

- Flash only through the `just` recipes: they write `app1` and `otadata` and nothing
  else, after checking the partition table.
- Never write `nvs` (the stock Wi-Fi credentials), `factory`, `app0` (the stock
  firmware) or the RTC. On the SD card, the app writes `cute-display.conf` and nothing
  else; the hardware test writes nothing.
- `just fw-stock` must always bring the stock firmware back.

## Commands

```sh
just test        # host tests
just lint        # clippy, host and ESP32
just preview     # render an app screen to a PNG (system, counter, echo, ping)
just fw          # build the app image, flash it into app1, watch the log
just fw hwtest   # the same with the hardware test image
just fw-stock    # boot the stock firmware again
```

The ESP32 build needs the `esp` toolchain (`espup`, sourcing `~/export-esp.sh`),
`espflash`, `ldproxy` and `just`.

## Before finishing a change

`just test` and `just lint` pass with no warnings; DESIGN.md and docs/hardware.md still
tell the truth.
