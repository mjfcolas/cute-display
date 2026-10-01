import hashlib
import json
import unittest
from urllib.error import HTTPError

from cute_display_installer.updating import Entry
from cute_display_installer.releases import LATEST, Release, ReleaseError, checked, image_assets, latest_release

IMAGE = b'\xe9' + b'cute-display' * 100
NAME = 'cute-display-2026.9.0.bin'
CHECKSUM = f'{hashlib.sha256(IMAGE).hexdigest()}  {NAME}\n'.encode()


def asset(name):
    return {'name': name, 'browser_download_url': f'https://example.test/{name}'}


RELEASE = {'tag_name': 'v2026.9.0', 'assets': [
    asset(NAME), asset(f'{NAME}.sha256'), asset('cute_display_installer-2026.9.0-py3-none-any.whl')]}


class Assets(unittest.TestCase):
    def test_the_image_and_its_checksum_are_found_among_the_assets(self):
        self.assertEqual(image_assets(RELEASE), (NAME, f'https://example.test/{NAME}',
                                                 f'https://example.test/{NAME}.sha256'))

    def test_a_release_without_exactly_one_image_and_its_checksum_is_refused(self):
        for assets in ([asset(NAME)], [], [asset(NAME), asset(f'{NAME}.sha256'), asset('cute-display-2026.9.1.bin')]):
            with self.assertRaisesRegex(ReleaseError, 'one image and its .sha256'):
                image_assets({'tag_name': 'v2026.9.0', 'assets': assets})


class Checksum(unittest.TestCase):
    def test_an_image_that_matches_passes(self):
        self.assertEqual(checked(NAME, IMAGE, CHECKSUM), IMAGE)

    def test_a_damaged_image_is_refused(self):
        with self.assertRaisesRegex(ReleaseError, 'did not arrive whole'):
            checked(NAME, IMAGE[:-1], CHECKSUM)

    def test_a_checksum_for_another_file_or_unreadable_is_refused(self):
        with self.assertRaisesRegex(ReleaseError, 'is for other.bin'):
            checked(NAME, IMAGE, f'{hashlib.sha256(IMAGE).hexdigest()}  other.bin\n'.encode())
        with self.assertRaisesRegex(ReleaseError, 'does not read'):
            checked(NAME, IMAGE, b'<html>not found</html>')


class OldestInstaller(unittest.TestCase):
    def test_an_installer_older_than_the_oldest_the_release_names_is_refused(self):
        release = Release(NAME, IMAGE, (), '2026.10.0')
        self.assertIn('needs installer 2026.10.0 or newer, and this one is 2026.9.1',
                      ' '.join(release.installer_refusals('2026.9.1')))
        self.assertEqual(release.installer_refusals('2026.10.0'), [])
        self.assertEqual(release.installer_refusals('2026.11.0'), [])

    def test_a_developer_installer_or_a_release_naming_none_refuses_nothing(self):
        self.assertEqual(Release(NAME, IMAGE, (), '2026.10.0').installer_refusals('2026.9.2.dev3+gabc1234'), [])
        self.assertEqual(Release(NAME, IMAGE, ()).installer_refusals('2026.9.1'), [])


class Latest(unittest.TestCase):
    def test_the_latest_release_image_comes_checked(self):
        served = {LATEST: json.dumps(RELEASE).encode(), f'https://example.test/{NAME}': IMAGE,
                  f'https://example.test/{NAME}.sha256': CHECKSUM}
        reported = []
        self.assertEqual(latest_release(served.__getitem__, reported.append), Release(NAME, IMAGE, ()))
        self.assertEqual(len(reported), 1)

    def test_its_updating_file_comes_with_it(self):
        release = dict(RELEASE, assets=RELEASE['assets'] + [asset('UPDATING.md')])
        served = {LATEST: json.dumps(release).encode(), f'https://example.test/{NAME}': IMAGE,
                  f'https://example.test/{NAME}.sha256': CHECKSUM,
                  'https://example.test/UPDATING.md': '# Updating\n\n## 2026.9.0\n\n- The alarm is set again.\n'.encode()}
        self.assertEqual(latest_release(served.__getitem__, lambda _: None).updating_entries,
                         (Entry('2026.9.0', '- The alarm is set again.'),))

    def test_its_oldest_installer_comes_with_it(self):
        release = dict(RELEASE, assets=RELEASE['assets'] + [asset('oldest-installer.txt')])
        served = {LATEST: json.dumps(release).encode(), f'https://example.test/{NAME}': IMAGE,
                  f'https://example.test/{NAME}.sha256': CHECKSUM,
                  'https://example.test/oldest-installer.txt': b'2026.9.0\n'}
        self.assertEqual(latest_release(served.__getitem__, lambda _: None).oldest_installer, '2026.9.0')

    def test_no_release_yet_is_said(self):
        def fetch(url):
            raise HTTPError(url, 404, 'Not Found', {}, None)

        with self.assertRaisesRegex(ReleaseError, 'no release yet'):
            latest_release(fetch, lambda _: None)

    def test_another_refusal_is_left_for_the_caller_to_tell(self):
        def fetch(url):
            raise HTTPError(url, 403, 'rate limit exceeded', {}, None)

        with self.assertRaises(HTTPError):
            latest_release(fetch, lambda _: None)


if __name__ == '__main__':
    unittest.main()
