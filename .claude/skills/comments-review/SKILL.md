---
name: comments-review
description: Review the comments and the docs of this repository against AGENTS.md rules 1 and 5.
---

# Reviewing comments and docs

## Comments

The rule is AGENTS.md's rule 1: comment only what the semantics failed to convey. Judge
each comment against the code it sits on.

For every comment in scope (`//`, `///`, `//!`, `#` in scripts and the `justfile`), ask
in this order; the first "yes" is the finding.

1. **Commented-out code?** Remove it.
2. **History?** "used to", "was changed", "now", "previously", "no longer", "fixed",
   "new": the code says what is, git says what was. Remove it.
3. **Restates the code?** It tells nothing the signature, the names and the body do not.
   Remove it.
4. **Explains *what* the code does?** Propose the rename, the type or the extraction that
   makes it unnecessary, with the new name.
5. **An invariant a type could carry?** "must be ≤ 100", "never empty", "true means…":
   propose the type.
6. **Stale?** It no longer matches the code. Quote both.
7. **Duplicated?** The same fact in a module comment and in `docs/` or `DESIGN.md`: the
   code is the reference, the doc goes (rule 5).

A comment that survives all seven is what the rule asks for: a measured hardware
behaviour, a non-obvious constraint, the reason something surprising is correct. Do not
report it, and do not ask for comments on code that reads well without one. When unsure
whether a comment holds a real hardware constraint, check `docs/hardware.md`; still
unsure, report a question, not a finding.

The missing side is a finding too: surprising code with no reason given (a magic delay,
a register write order, a workaround).

## Docs

The rule is AGENTS.md's rule 5: docs work by reference, and the code is the reference
for behaviour. In scope: the Markdown files a change touches, and those that describe
code the change touches (`README.md`, `DESIGN.md`, `AGENTS.md`, `docs/`).

1. **Stale?** A name, path, module, conf key, value or `just` recipe that no longer
   matches the code, or a broken link. Quote both.
2. **History?** Same as for comments: remove it.
3. **Repeats the code?** Behaviour, timings, algorithms, protocol steps a module comment
   or the code already holds. Remove it, or link to the module.
4. **Duplicated between docs?** The same fact in two files: keep it where it belongs,
   link from the other.
5. **In the wrong file?**
   - The root `README.md` describes instead of naming and linking.
   - A part's `README.md` holds design, or details beyond what it does, its controls and
     its conf files.
   - A `DESIGN.md` describes behaviour, or holds nothing the code does not.
   - The root `DESIGN.md` holds one part's details instead of linking to it.
6. **Too big?** A file growing past one subject: split it and link.
7. **Wordy?** Prose where short bullet points would do, in a README for people.

A missing doc is a finding too: a new app, conf file or `just` recipe that no README
mentions.

## Tags

`comment: dead code`, `comment: history`, `comment: restates`,
`comment: what → rename`, `comment: invariant → type`, `comment: stale`,
`comment: duplicated`, `comment: missing why`, `doc: stale`, `doc: history`,
`doc: repeats code`, `doc: duplicated`, `doc: wrong file`, `doc: too big`, `doc: wordy`,
`doc: missing`.
