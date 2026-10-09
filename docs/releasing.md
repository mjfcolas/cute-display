# Releasing

A release is the app image of one commit, attached to a GitHub release.

Versions are dates: `v<year>.<month>.<n>`, as git tags. `n` counts the releases of the
month, from 0 (`v2026.9.0`, then `v2026.9.1`, then `v2026.10.0`). There is no leading
zero, because the file name of the installer's Python package (its wheel) would remove
it. The version is written only in the tag:
the images and the installer read it from git.

1. In [`CHANGELOG.md`](../CHANGELOG.md) and [`UPDATING.md`](../UPDATING.md), which were
   written during the work, change `## Unreleased` to `## <version>`. `just release`
   refuses a tag while `## Unreleased` is there. If the image needs an installer newer
   than the one in [`oldest-installer.txt`](../tools/release/oldest-installer.txt) (for
   example, when a file that the setup writes on the card changes), put the new version
   there. Commit.
2. Tag this commit: `git tag v<version>`.
3. Run `just release`. Uncommitted changes are included if there are any, and the
   release still has the tag's version. It writes into `release/`:
   - the image `cute-display-<version>.bin` and its `.sha256`;
   - the installer `cute_display_installer-<version>-py3-none-any.whl`;
   - `install.sh` and `install.cmd`, which install this wheel and run the setup;
   - `UPDATING.md` and `oldest-installer.txt`.
4. Push the tag, and attach the seven files to the GitHub release of this tag. Its notes
   are the version's changelog. The lines in [installing](install.md#in-short) download
   `install.sh` and `install.cmd` from the latest release, so they do not change from
   one release to the next.

A release has exactly one `cute-display-*.bin` and its `.sha256`: the installer's
`install` command takes the ones of the latest release. It also takes the latest
release's `UPDATING.md`, whatever the installer's own version is. So an entry written
after the release is out is never shown. An installer older than the version in
`oldest-installer.txt` refuses the release. Installers from before this check (2026.9.1
and older) cannot know.

On a commit without a tag, `just release` makes a snapshot of the next version instead,
`<version>-snapshot`. It is for testing, not for publishing: its `install.sh` and
`install.cmd` install the wheel from `release/` on this computer.
