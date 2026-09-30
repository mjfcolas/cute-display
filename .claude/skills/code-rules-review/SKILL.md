---
name: code-rules-review
description: Review this repository's code, comments and docs aside, against AGENTS.md and DESIGN.md.
---

# Reviewing code against the rules

The rules are in AGENTS.md, the dependency graph in DESIGN.md. For each rule, what to
look for:

- **Naming** (rule 1): `Manager`, `handle`, `utils`, `data`, `set_val`; a boolean
  parameter where an enum would say what the caller wants; a value with an invariant
  carried by a bare number.
- **Layers** (rule 2): a `use` or a `Cargo.toml` dependency pointing the wrong way in
  DESIGN.md's graph; a `hal` type in `domain`; hardware in `ui`; a pin outside
  `firmware`.
- **Host tests** (rule 3): logic in `drivers` or `firmware` that touches no peripheral
  and has no host test; a HAL consumer tested without a test double; new logic without a test.
- **Panics** (rule 4): `unwrap`, `expect`, `[i]`, `panic!` outside tests, or an
  `#[allow]` that silences them. A hardware failure that is not a `hal::Fault`.
- **The SD card** (rule 6): a write or a removal outside `cute-display/`.
- **NVS** (rule 7): a key outside the `cute_display` namespace; an erase of the
  partition; an ESP-IDF component left to write NVS on its own.

Tags: `naming`, `layers`, `tests`, `panic`, `sd card`, `nvs`.
