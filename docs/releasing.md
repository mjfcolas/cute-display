# Releasing

A release is the app image of one commit, attached to a GitHub release.

Versions are dates, `v<year>.<month>.<n>` as git tags, `n` counting the month's releases
from 0 (`v2026.9.0`, then `v2026.9.1`, then `v2026.10.0`), with no leading zero: the
wheel's name would drop it. The tag is the only place the version is written: the images
and the installer take it from git.

1. Say what changed under the new version in [`CHANGELOG.md`](../CHANGELOG.md), and commit.
2. Tag that commit: `git tag v<version>`.
3. `just release`, uncommitted changes included if any: the release still says the tag's
   version. In `release/`, the image `cute-display-<version>.bin`, its `.sha256`,
   the installer `cute_display_installer-<version>-py3-none-any.whl`, and `install.sh`
   and `install.cmd`, which install that wheel and run the setup.
4. Push the tag, and attach the five files to the GitHub release of that tag, its notes
   being the version's changelog. The lines in [installing](install.md#in-short) fetch
   `install.sh` and `install.cmd` from the latest release, so they need no change from
   one release to the next.

A release carries exactly one `cute-display-*.bin` and its `.sha256`: the installer's
`install` takes the latest release's.

On a commit without a tag, `just release` makes a snapshot of the next version instead,
`<version>-snapshot`, to try out and not to publish: its `install.sh` and `install.cmd`
install the wheel from `release/` on this computer.
