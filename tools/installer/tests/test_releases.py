import hashlib
import json
import unittest
from urllib.error import HTTPError

from cute_display_installer.releases import LATEST, ReleaseError, checked, image_assets, latest_image

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


class Latest(unittest.TestCase):
    def test_the_latest_release_image_comes_checked(self):
        served = {LATEST: json.dumps(RELEASE).encode(), f'https://example.test/{NAME}': IMAGE,
                  f'https://example.test/{NAME}.sha256': CHECKSUM}
        reported = []
        self.assertEqual(latest_image(served.__getitem__, reported.append), (NAME, IMAGE))
        self.assertEqual(len(reported), 1)

    def test_no_release_yet_is_said(self):
        def fetch(url):
            raise HTTPError(url, 404, 'Not Found', {}, None)

        with self.assertRaisesRegex(ReleaseError, 'no release yet'):
            latest_image(fetch, lambda _: None)

    def test_another_refusal_is_left_for_the_caller_to_tell(self):
        def fetch(url):
            raise HTTPError(url, 403, 'rate limit exceeded', {}, None)

        with self.assertRaises(HTTPError):
            latest_image(fetch, lambda _: None)


if __name__ == '__main__':
    unittest.main()
