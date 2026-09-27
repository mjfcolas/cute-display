# Releasing

A release is the app image of one commit, attached to a GitHub release.

Versions are dates: `<year>.<month>.<n>`, `n` counting the month's releases from 0
(`2026.9.0`, then `2026.9.1`, then `2026.10.0`). No leading zero: Cargo refuses `2026.09.0`.

1. Set the new version in `src/app/Cargo.toml` and `src/firmware/Cargo.toml`: the build
   refuses them different.
2. Say what changed under that version in [`CHANGELOG.md`](../CHANGELOG.md).
3. Commit, then `just release`: `release/cute-display-<version>.bin` and its `.sha256`.
4. Tag the commit `v<version>`, push the tag, and attach both files to the GitHub
   release of that tag, its notes being the version's changelog.
