"""Cute Display's releases on GitHub: the latest one's image, checked against its SHA-256
before anything is written, what to know before updating to it, and the oldest installer
that may install it. docs/releasing.md says what a release carries."""
import hashlib
import json
import re
from dataclasses import dataclass
from urllib.error import HTTPError

from . import updating, versions
from .web import fetch as fetch_from_web

REPOSITORY = 'mjfcolas/cute-display'
LATEST = f'https://api.github.com/repos/{REPOSITORY}/releases/latest'
PAGE = f'https://github.com/{REPOSITORY}/releases'
_IMAGE_NAME = re.compile(r'^cute-display-.+\.bin$')
OLDEST_INSTALLER = 'oldest-installer.txt'
# A line of sha256sum: the digest, then the file's name, `*` before it in binary mode.
_CHECKSUM_LINE = re.compile(r'^([0-9a-fA-F]{64}) [ *](\S+)\s*$')


class ReleaseError(Exception):
    pass


@dataclass(frozen=True)
class Release:
    name: str
    image: bytes
    updating_entries: tuple[updating.Entry, ...]
    oldest_installer: str | None = None

    def installer_refusals(self, installer_version):
        """A developer's installer, between releases, is never refused."""
        if self.oldest_installer is None:
            return []
        oldest, this = versions.release(self.oldest_installer), versions.release(installer_version)
        if oldest is None or this is None or this >= oldest:
            return []
        return [f'it needs installer {self.oldest_installer} or newer, and this one is {installer_version}: '
                'the line that installed it, run again, takes the latest.']


def image_assets(release):
    """The image's and its checksum's download URLs, from a release as GitHub's API
    describes it."""
    assets = {asset['name']: asset['browser_download_url'] for asset in release.get('assets', [])}
    images = [name for name in assets if _IMAGE_NAME.match(name)]
    if len(images) != 1 or f'{images[0]}.sha256' not in assets:
        raise ReleaseError(f'release {release.get("tag_name", "?")} does not carry one image and its '
                           f'.sha256; see {PAGE}')
    return images[0], assets[images[0]], assets[f'{images[0]}.sha256']


def checked(name, contents, checksum_file):
    """`contents` if the checksum file, as sha256sum writes it, vouches for it."""
    line = _CHECKSUM_LINE.match(checksum_file.decode(errors='replace'))
    if not line:
        raise ReleaseError(f'{name}.sha256 does not read as a checksum')
    expected, listed = line.groups()
    if listed != name:
        raise ReleaseError(f'{name}.sha256 is for {listed}')
    if hashlib.sha256(contents).hexdigest() != expected.lower():
        raise ReleaseError(f'{name} did not arrive whole: its SHA-256 differs; try again')
    return contents


def latest_release(fetch=fetch_from_web, report=print):
    """The latest release's image, checked, its UPDATING.md's entries and its oldest
    installer, none in a release made before they were attached."""
    try:
        release = json.loads(fetch(LATEST))
    except HTTPError as error:
        if error.code == 404:
            raise ReleaseError(f'no release yet on {PAGE}') from None
        raise
    name, image_url, checksum_url = image_assets(release)
    report(f'Downloading {name} from {PAGE}...')
    image = checked(name, fetch(image_url), fetch(checksum_url))
    assets = {asset['name']: asset['browser_download_url'] for asset in release.get('assets', [])}

    def text(asset_name):
        return fetch(assets[asset_name]).decode(errors='replace') if asset_name in assets else None

    updating_text, oldest_installer = text(updating.FILE), text(OLDEST_INSTALLER)
    return Release(name, image, () if updating_text is None else updating.entries(updating_text),
                   None if oldest_installer is None else oldest_installer.strip())
