import pathlib
import re
import tomllib
import unittest

INSTALLER = pathlib.Path(__file__).resolve().parents[1]
APP_MANIFEST = INSTALLER.parents[1] / 'src' / 'app' / 'Cargo.toml'


class Version(unittest.TestCase):
    def test_the_installer_is_released_with_the_app_image_of_the_same_version(self):
        installer = tomllib.loads((INSTALLER / 'pyproject.toml').read_text())['project']['version']
        app = re.search(r'^version = "(.*)"', APP_MANIFEST.read_text(), re.MULTILINE).group(1)
        self.assertEqual(installer, app)


if __name__ == '__main__':
    unittest.main()
