# AGENTS.md

Firmware for the Habity bedside clock (ESP32-S3, e-paper, wheel, buttons, RTC, SD,
speaker, lights).

- [`DESIGN.md`](DESIGN.md): the architecture.
- [`docs/hardware.md`](docs/hardware.md): the board.

## Rules

### 1. Naming, semantics and architecture carry the meaning

- A name says what a thing *is* or *does*, in the terms of the product or the hardware:
  `Brightness::adjusted_by`, `PushButton::take_presses`, `Uc8253`, not `set_val`,
  `handle`, `Manager`, `utils`.
- Types hold the invariants, not comments: a `Brightness` cannot be more than 100 %. A
  `Redraw` says what the caller wants, where a boolean would not.
- The modules and crates follow the functions of the product. A person who reads the
  tree should understand what the product does, and where each concept is.
- **Comment only what the code cannot say by itself**: a measured hardware behaviour, a
  constraint that is not obvious, the reason why something surprising is correct. Never
  repeat the code, never tell its history ("used to", "was changed because"), never
  leave code in comments. If code needs a comment to explain *what* it does, rename or
  restructure it first.

### 2. Layered architecture, SOLID

[`DESIGN.md`](DESIGN.md) describes it. In short: `domain` ← `infrastructure` → `hal` ←
`drivers`, and `ui` → `domain`. `app` wires the app image on any hardware. The
composition roots are `firmware` (the board) and `simulator` (a computer).
Dependencies only go in the directions that DESIGN.md gives. A new dependency in another
direction is a design change: change DESIGN.md first.

### 3. Every piece of logic is tested on the host

- Only `drivers` (its ESP32 half) and `firmware` need the device. Everything else builds
  and is tested with a standard toolchain: `just test`.
- Logic in a driver that does not use a peripheral (a memory layout, a register decoder,
  a debouncer) is written so that it compiles and is tested on the host.
- Code that uses a HAL contract is tested with test doubles of that contract.
- Each test goes in the lowest tier of [the pyramid](docs/testing/README.md) that can
  fail for the right reason.
- The name of a test double starts with its kind, in Martin Fowler's terms, then says
  what it replaces (`StubInternet`, `FakeFileStorage`). A `Dummy` is passed but never
  used. A `Stub` gives answers set in advance. A `Fake` works, but in a simpler way. A
  stub that also records how it was called (Fowler's spy) is still a `Stub`. When the
  kind is not enough, a word in the middle says what is different: `StubFailingLight`.
- In a crate that runs on the clock, what only the test image or the simulator needs
  (the test image's network that goes to stubs, an answer from an NTP server) goes in
  its `testing` module. Where test doubles go:
  [the pyramid](docs/testing/README.md#where-a-test-goes).

### 4. No panics on the device

The lints forbid the panic family (`unwrap_used`, `indexing_slicing`, …), except in
tests. A hardware failure is a `hal::Fault` value. It ends up on the screen or in the
log, and the device keeps running.

### 5. Documentation works by reference

- Docs and comments are written in plain English: short sentences, common words, no
  idioms or literary turns. Many readers are not native speakers, and idioms make the
  text harder for them to understand.
- Small files, one subject each, linked from where they are needed; never one large
  file. `README.md` and `DESIGN.md` at the root are indexes that point to the details.
- Each app (`docs/apps/<app>/`), the maintenance console, the hardware test and the
  simulator has a `README.md` for people: what it does, its controls and its conf files,
  in short bullet points. A `DESIGN.md` next to it holds only the design decisions that
  no module comment holds.
- The root `README.md` is for people who own the clock. It names and links each part;
  what a part does is in its own README. What developers need starts at
  `docs/development.md`.
- The code is the reference for behaviour and details. Docs do not repeat what a name, a
  type or a module comment already says.
- The apps are listed only once, in the root `README.md`. Everywhere else, docs and
  diagrams speak of apps in general, so that a new app does not change them.

### 6. On the SD card, each image keeps its files in its own directory

The composition root gives each image a directory: `cute-display/` for the app image,
`cute-display-e2e/` for the test image. An image and its maintenance console write
nowhere else on the card. So all the files of an image are in one place, and the tests
never touch the files that the clock keeps.

### 7. In NVS, our keys are in the `cute_display` namespace

The `nvs` partition belongs to Habity's firmware: its settings and the Wi-Fi
credentials, which it needs when the device starts it again.

- We store our data on the SD card first. We use NVS only for what cannot wait for the
  card.
- Our keys go in the `cute_display` namespace, and nowhere else. The test image's keys go
  in `cute_disp_e2e`, because an NVS name has 15 characters at most.
- Never erase the partition, even when `nvs_flash_init` asks for it: that would delete
  Habity's settings.
- ESP-IDF components that would write to NVS by themselves are configured not to (the
  Wi-Fi driver without NVS, PHY calibration not stored).

### 8. The changelog is written with the change

[`CHANGELOG.md`](CHANGELOG.md) and [`UPDATING.md`](UPDATING.md) are for people who own
the clock. Each change that they would notice goes in them, under `## Unreleased`, in
the same commit.

- `CHANGELOG.md`: what they can see or do that is new, in their words: an app, a
  setting, a control, a bug they had. Not how it is made: tests, tools, refactors.
- `UPDATING.md`: what an update loses or changes for them (a setting lost, a file moved,
  a setup question to answer again). The installer shows it before it updates.

## Commands

In [docs/development.md](docs/development.md#commands), with what they need.

## Before finishing a change

`just test` and `just lint` pass with no warnings. The docs that a change touches are
still true. The changelog says what the change means for people ([rule 8](#8-the-changelog-is-written-with-the-change)).

The [`rules-reviewer`](.claude/agents/rules-reviewer.md) agent reviews a change against
these rules.
