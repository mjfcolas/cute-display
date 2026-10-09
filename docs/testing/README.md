# Testing

The tests form a pyramid: many fast tests at the bottom, and a few slow tests at the
top. Each test goes in the lowest tier that can fail for the right reason. A scenario is
added to the device's end-to-end tier only for a risk that exists only on the device,
and that the other tiers do not cover.

| Tier | Where | Proves | Does not prove | Status |
| --- | --- | --- | --- | --- |
| Unit and component | The tests of each crate, with the test doubles of `hal_testing` and `domain_testing` | The domain's rules, the gestures, what a screen shows, a driver's decoders | The wiring, the threads, time passing | In place |
| Contract | The `check_*` functions of `hal_testing` and `domain_testing`. Each one runs on every double and every host implementation of its contract | That the doubles behave like the real implementations | What the chips do | In place |
| Rendering | A screen's UI state is drawn and compared with `references/<name>.png` in its crate, by [`ui_testing::references`](../../src/engine/ui_testing/src/references.rs) | That a UI state is drawn as expected | That it is the right UI state | In place |
| Integration | The whole app image, `app::run`, in memory, with test doubles of the hardware and a clock that the test moves forward | The threads, the wiring, long periods of time, faults | The real process and its I/O | In place |
| End-to-end, simulator | [Scenarios](../../tools/e2e/README.md) that control the simulator, without a window, through the [maintenance console](../maintenance/README.md) | What a person does, from the controls to the screen, the lights, the sound and the card | The hardware | In place |
| End-to-end, device | The scenarios that are fast enough in real time, on the device running the test image. The test image has its own directory on the card. It gets the web and the time from stubs on the local network. Its clock stays where the scenario set it | Memory, stacks, TLS, the card, the RTC, the speaker, real time | The button pins and the screen: the [hardware test](../hwtest/README.md) covers them | In place |

## Where a test goes

- Use the lowest tier first: a rule of the domain is a unit test, never an end-to-end
  test.
- A test double of a contract goes in `hal_testing` or `domain_testing`, with the check
  that it and every implementation must pass. A double that breaks a contract on
  purpose, or that a test watches together with other doubles, stays in its test. Its
  name gives its kind, then what it replaces:
  [AGENTS.md](../../AGENTS.md#3-every-piece-of-logic-is-tested-on-the-host).
- A test that needs time to pass (a night before the alarm, a retry in ten minutes, a
  fetch every hour) is an integration test, with the clock that the test moves forward.
- A scenario runs on the device only because it covers a risk that exists only on the
  device. Passing on the simulator is not a reason.
- Screens are tested on what they show, their UI state. Pixels are tested only in the
  rendering tier.

## Commands

- `just test`: the unit, component, contract, rendering and integration tests, for the
  Rust crates and the Python tools.
- `UPDATE_REFERENCES=1 cargo test`: the frames that the tests draw become the new
  rendering references. Look at them before you commit.
- `just lint`: clippy, host and ESP32.
- `just e2e simulator [pytest arguments]`: the end-to-end scenarios on the simulator.
- `just e2e device [pytest arguments]`: the scenarios marked `device`, on the device. It
  flashes the test image, then flashes the app image again.
- `just preview <screen>`: a screen as a PNG, to look at.
- `just remote-sim …` and `just remote …`: use the console by hand, on the simulator or
  the device.
