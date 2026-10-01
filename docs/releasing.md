# Releasing

A release is the app image of one commit, attached to a GitHub release.

Versions are dates, `v<year>.<month>.<n>` as git tags, `n` counting the month's releases
from 0 (`v2026.9.0`, then `v2026.9.1`, then `v2026.10.0`), with no leading zero: the
wheel's name would drop it. The tag is the only place the version is written: the images
and the installer take it from git.

1. In [`CHANGELOG.md`](../CHANGELOG.md) and [`UPDATING.md`](../UPDATING.md), written as
   the work went, `## Unreleased` becomes `## <version>`; `just release` refuses a tag
   while it is there. If the image needs an installer newer than the one
   [`oldest-installer.txt`](../tools/release/oldest-installer.txt) names, as when a file
   the setup writes on the card changes, put the new version there. Commit.
2. Tag that commit: `git tag v<version>`.
3. `just release`, uncommitted changes included if any: the release still says the tag's
   version. In `release/`, the image `cute-display-<version>.bin`, its `.sha256`,
   the installer `cute_display_installer-<version>-py3-none-any.whl`, `install.sh`
   and `install.cmd`, which install that wheel and run the setup, `UPDATING.md`, and
   `oldest-installer.txt`.
4. Push the tag, and attach the seven files to the GitHub release of that tag, its notes
   being the version's changelog. The lines in [installing](install.md#in-short) fetch
   `install.sh` and `install.cmd` from the latest release, so they need no change from
   one release to the next.

A release carries exactly one `cute-display-*.bin` and its `.sha256`: the installer's
`install` takes the latest release's. Its `UPDATING.md` too, whatever version the
installer itself is: an entry written once the release is out is not seen. An installer
older than its `oldest-installer.txt` refuses it; one older than this check, 2026.9.1 and
before, cannot tell.

On a commit without a tag, `just release` makes a snapshot of the next version instead,
`<version>-snapshot`, to try out and not to publish: its `install.sh` and `install.cmd`
install the wheel from `release/` on this computer.
