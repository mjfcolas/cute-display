# Testing plan

> **Not done yet.** Steps 1 and 2 are done; **step 3 is the current one**. This file
> is updated as each step lands, and goes once the last one has: the
> [pyramid](README.md) and the code are then the reference.

## Where we start

- About 335 Rust tests, mostly unit, some of components (the alarm's `Bench`, the `Shell`
  with the `SystemScreen`), and about 160 Python tests of the tools; `just test` runs them.
- Nothing tests `app::run`, its threads or `network::start`.
- A screen draws from its private state: it can only be checked by its pixels, or by
  strings built beside the drawing (`status_line`, `day_line`).
- The simulator needs a window, uses the real Internet (open-meteo, adsb.fi, NTP), starts
  at the computer's time, and NTP overrides `clock set`; `--speed` scales real time.
- The console already drives both the simulator and the device, and `tools/link` is its
  client. The `raw-work` branch holds a first end-to-end attempt to take from: pytest in
  `tools/e2e`, `--headless --offline --time`, `sim_only`/`device_only`/`sets_clock`
  markers, crashes in the log failing a scenario.

## Decisions

- Screens get a UI state; tests check what a screen shows through it.
- End-to-end scenarios check the UI state, which the console reads out; reference PNGs
  belong to the rendering tier.
- The device for end-to-end is the clock in daily use, in a test mode that restores its
  card, its clock and its state afterwards.
- No CI for now; the `just` recipes are what a CI would run.

## What is missing

### A. A UI state, in `ui`

- Each screen splits into `ui_state(&self) -> <Screen>UiState`, plain data of its own
  projected from what the screen holds, and a pure drawing of that state;
  `AppScreen::draw` draws `self.ui_state()`. What a person sees is said as they see it:
  each row carries its `Mark`, chosen or plain, rather than the state an index.
- In order: `SystemScreen`, the alarm, the weather, the radar. Their unit tests move to
  the UI states, `status_line`/`day_line` with them; the pixel tests (nothing outside
  the area, where a mark or a line is drawn) stay, in the rendering tier, drawing UI
  states made by hand.

### B. The description reaches the console

- `ui::Description`, generic, what leaves the UI: an ordered list of
  `{ name, value, selected }`, made from each UI state. `AppScreen::describe`, empty by
  default while screens move over. State and description come from the same
  `ui_state()`.
- The `Shell` describes what is in front: the app, the system screen open or not, then
  the screen's description.
- A UI state holds the text as shown, but for what the toolkit draws from domain data
  (the time in big digits, a forecast's icons): the description turns those into text.
- `maintenance` sees `hal` only: plain data on the `hal` side, as `ControlsSample` is on
  the `ui` side (`hal::display::ScreenText`, lines `name value [*]`), and a contract that
  receives it (`ScreenReader`, like an accessibility screen reader). Names settled in
  DESIGN.md at that step.
- `app::Presentation::tick` turns the `Description` into `ScreenText` and hands it over
  right after `panel.show`.
- `maintenance::Observation` keeps the last one with its `times_shown`; a `describe`
  request answers `text` lines then `ok <times_shown>`; `remote describe` in `tools/link`.
- `docs/maintenance/DESIGN.md`: the console sees what a person reads, never the domain.

### C. Shared test doubles and their contracts — done

- `hal_testing` and `domain_testing` hold the test doubles, and a `check_*` for each
  contract whose behaviour a double must keep: storage, steady clock, RTC, button,
  wheel, light; an app's files, the time keeper. Each runs on the doubles and on every
  host implementation.
- The other contracts have no check: what they promise (a panel may redraw more than
  asked, a speaker plays, a place or an answer is given) leaves a double nothing to
  keep.
- Still to come, with the step that first needs them (5 and 6): `StubHttpClient`,
  answering recorded responses by URL, and `StubUdpClient`, answering SNTP with a given
  time.
- `domain`'s own tests keep their doubles: a crate cannot use one built on itself.

### D. Virtual time, and the app image in a test

- `VirtualSteady`: `sleep` records a deadline and parks the thread; `advance(d)` wakes
  the sleepers one at a time, earliest first, each until it parks again: serial and
  deterministic. Dropped, it leaves the threads parked, so `app::run` keeps returning
  `Infallible`.
- To try first: the speaker's thread waits on a channel, not on `sleep`. Either the stub
  speaker sleeps on the virtual clock for the length of the sound, or sound is asserted
  as "eventually".
- `TestHardware`, an `app::Hardware` in `src/app/tests/`: the doubles of C, driven by
  `maintenance::Remote`, observed by `maintenance::Observation` and the `ScreenText`,
  with the apps of `catalog`.

### E. A deterministic simulator

- `--headless`: no window; the controls are the console's alone.
- `--time <unix>`: the RTC starts there, and NTP answers that time (`StubUdpClient`).
- `--web <dir>`: HTTP answered from recorded responses (`StubHttpClient`); `--record <dir>`
  records them from the real Internet; `--offline`: no network.

### F. Synchronising with the console

- `tap` and `turn` answer once the image has taken them (`take_presses`,
  `take_detents`), still at the HAL.
- Clients wait for a description to match, with a timeout; a `settled` request only if
  negative assertions need one.
- `restart`, on a `hal::system` contract: `esp_restart` on the device, the link
  reconnecting; the simulator ends and the harness starts it again.
- `tools/link` keeps the log lines it does not consume, for diagnostics and to catch a
  crash or a reboot.

### G. The end-to-end harness, `tools/e2e`, pytest

- A `target` fixture: `Simulator` or `Device`.
- `Simulator`, one per scenario: a copy of `tools/e2e/cards/standard/`, `--headless
  --speed N --time T --web tools/e2e/web/`.
- `Device`, one per session, in test mode: keep `cute-display/`'s conf files, put the
  test card's (no `wifi.conf`: offline, no NTP), `clock set T`, `restart`; at the end,
  failed or not, put them back, set the real time, `restart`.
- Scenarios see the console alone: controls, `describe`, lights, sound, clock, card.
- Markers: `sim_only` (recorded data: weather, radar), `device_only` (a real fetch:
  `wifi.conf` back, a forecast eventually arrives), `device_smoke` (what runs on the
  device).
- A scenario fails if the log shows a panic or an unexpected restart.
- `just e2e [pytest arguments]` on the simulator, `just e2e-device` (monitor closed).

## Steps

Each lands on its own, with `just test` and `just lint` passing and the docs it touches
still true.

1. **Done.** The strategy: [the pyramid](README.md), this plan, linked from
   `docs/development.md` and AGENTS.md.
2. **Done.** Shared test doubles and contracts (C).
3. **Current.** The UI state (A): `SystemScreen`'s (done), the alarm's (done), the
   weather's, the radar's; then rendering references in Rust, `<name>.seen.png` beside
   them on a difference, `BLESS=1` to take them anew.
4. The description to the console (B): `describe`, `remote describe`; the `drive` skill
   reads the screen as text.
5. A trial of the virtual clock, then the integration tier (D): starting with and
   without a card; a night until the alarm (in front, light and sound, snooze, stop);
   NTP setting the RTC, failing, retrying ten minutes later; a recorded forecast shown.
6. The deterministic simulator (E) and the console's synchronisation (F), but `restart`.
7. The end-to-end harness on the simulator (G), five to eight scenarios: start, switching
   apps, setting an alarm, ringing on time, the light, recorded weather, recorded radar,
   system settings kept on the card.
8. The device's test mode: `restart`, keeping and restoring, `just e2e-device`,
   `device_smoke`, a first run on the clock.

## Checking each step

- 3 and 4: `just preview <screen>` looks the same; `just remote-sim describe` reads what
  the screen shows.
- 5: the integration tests run fifty times in a row without a random failure.
- 7: `just e2e` passes twice alike, and a deliberate change to a screen fails the right
  scenario.
- 8: after `just e2e-device`, the conf files are those kept, the time is right, and the
  clock shows the app it started on.
