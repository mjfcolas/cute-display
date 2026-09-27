# AGENTS.md

Firmware for the Habity bedside clock (ESP32-S3, e-paper, wheel, buttons, RTC, SD,
speaker, lights).

- [`DESIGN.md`](DESIGN.md): the architecture.
- [`docs/hardware.md`](docs/hardware.md): the board.

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
`drivers`, `ui` → `domain`; `app` wires the app image on any hardware, and the
composition roots are `firmware` (the board) and `simulator` (a computer).
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

### 5. Documentation works by reference

- Small files, one subject each, linked from where they are needed; never one large
  file. `README.md` and `DESIGN.md` at the root are indexes that point to the details.
- Each app (`docs/apps/<app>/`), the maintenance console, the hardware test and the
  simulator has a `README.md` for people: what it does, its controls, its conf files, in
  short bullet points. A `DESIGN.md` beside it only for design decisions no module comment holds.
- The root `README.md` is for people who own the clock: it names and links, and what a
  part does is in its own README. What developers need starts at `docs/development.md`.
- The code is the reference for behaviour and details: docs do not repeat what a name,
  a type or a module comment already says.

### 6. On the SD card, ours is under `cute-display/`

The device and its maintenance console write nowhere else on the card, so everything
of ours is in one place.

### 7. In NVS, ours is in the `cute_display` namespace

The `nvs` partition is Habity's: its settings and Wi-Fi credentials, which it needs when
the device boots it again.

- What we keep goes on the SD card first; NVS only for what cannot wait for the card.
- Our keys go in the `cute_display` namespace, and nowhere else.
- Never erase the partition, not even when `nvs_flash_init` asks for it: that would
  wipe Habity's settings.
- ESP-IDF components that would write NVS on their own are configured not to (the Wi-Fi
  driver without NVS, PHY calibration not stored).

## Commands

In [docs/development.md](docs/development.md#commands), with what they need.

## Before finishing a change

`just test` and `just lint` pass with no warnings; the docs a change touches still tell
the truth.

The [`rules-reviewer`](.claude/agents/rules-reviewer.md) agent reviews a change against
these rules.
