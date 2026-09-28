# Releasing

A release is the app image of one commit, attached to a GitHub release.

Versions are dates: `<year>.<month>.<n>`, `n` counting the month's releases from 0
(`2026.9.0`, then `2026.9.1`, then `2026.10.0`). No leading zero: Cargo refuses `2026.09.0`.

1. Set the new version in `src/app/Cargo.toml`, `src/firmware/Cargo.toml` and
   `tools/installer/pyproject.toml`: the build and `just test` refuse them different.
2. Say what changed under that version in [`CHANGELOG.md`](../CHANGELOG.md).
3. Commit, then `just release`: in `release/`, the image `cute-display-<version>.bin`,
   its `.sha256`, the installer `cute_display_installer-<version>-py3-none-any.whl`, and
   `install.sh` and `install.cmd`, which install that wheel and run the setup.
4. Tag the commit `v<version>`, push the tag, and attach the five files to the GitHub
   release of that tag, its notes being the version's changelog. The lines in
   [installing](install.md#in-short) fetch `install.sh` and `install.cmd` from the latest
   release, so they need no change from one release to the next.

A release carries exactly one `cute-display-*.bin` and its `.sha256`: the installer's
`install` takes the latest release's.
