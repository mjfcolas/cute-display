# Testing

A pyramid: many fast tests at the bottom, a few slow ones on top, each test in the
lowest tier that can fail for the right reason. Not all tiers exist yet: the
[plan](plan.md) says which, and what comes next.

| Tier | Where | Proves | Does not prove | Status |
| --- | --- | --- | --- | --- |
| Unit and component | Each crate's tests, on the test doubles of `hal_testing` and `domain_testing` | The domain's rules, the gestures, what a screen shows, a driver's decoders | The wiring, the threads, time going by | In place |
| Contract | The `check_*` of `hal_testing` and `domain_testing`, each run on every double and every host implementation of its contract | That the doubles tell the truth | What the chips do | In place |
| Rendering | A screen's UI state drawn and compared with its crate's `references/<name>.png`, by [`ui_testing::references`](../../src/engine/ui_testing/src/references.rs) | That a UI state is drawn as expected | That it is the right UI state | In place |
| Integration | The whole app image, `app::run`, in memory on test doubles of the hardware and a virtual clock stepped by the test | The threads, the wiring, long spans of time, faults | The real process and its I/O | Planned |
| End-to-end, simulator | Scenarios driving the headless simulator through the [maintenance console](../maintenance/README.md) | What a person does, from the controls to the screen, the lights, the sound and the card | The hardware | Planned |
| End-to-end, device | The same scenarios on the device, in a test mode that puts its card and clock back | Memory, stacks, TLS, the card, the RTC, the speaker, real time | The button pins and the glass: the [hardware test](../hwtest/README.md) covers those | Planned |

## Where a test goes

- Lowest tier first: a rule of the domain is a unit test, never an end-to-end one.
- A test double of a contract goes in `hal_testing` or `domain_testing`, with the check
  it and every implementation pass; one that breaks a contract on purpose stays in its
  test. Its name is its kind, then what it stands for: [AGENTS.md](../../AGENTS.md#3-every-piece-of-logic-is-tested-on-the-host).
- Anything that needs time to pass (a night before the alarm, a retry in ten minutes, an
  hourly fetch) is an integration test, on the virtual clock.
- A scenario runs on the device because it covers a risk only the device has, not
  because it passes on the simulator.
- Screens are checked on what they show, their UI state; pixels only in the rendering
  tier.

## Commands

- `just test`: the unit, component, contract and rendering tests, the Rust crates and the
  Python tools.
- `UPDATE_REFERENCES=1 cargo test`: the frames drawn become the rendering references;
  look at them before committing.
- `just lint`: clippy, host and ESP32.
- `just preview <screen>`: a screen as a PNG, to look at.
- `just remote-sim …` and `just remote …`: the console by hand, on the simulator or the
  device.
