import ssl
import tempfile
import unittest
import urllib.error
import urllib.request
from pathlib import Path

from cute_display_e2e.authority import CERTIFICATE, KEY, make_authority_pem
from cute_display_e2e.stub_web_server import WEB, StubWebServer, e2e_conf, recorded_web

FORECAST = 'https://api.open-meteo.com/v1/forecast?latitude=48.8530&longitude=2.3499'


class WebServer(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.authority = Path(directory.name)
        certificate, key = make_authority_pem()
        (self.authority / CERTIFICATE).write_bytes(certificate)
        (self.authority / KEY).write_bytes(key)
        self.server = StubWebServer('127.0.0.1', port=0, web={FORECAST: b'{"current": {}}'}, authority=self.authority)
        self.addCleanup(self.server.close)

    def get(self, url_below_host):
        trusting_the_authority_alone = ssl.create_default_context(cafile=self.authority / CERTIFICATE)
        url = f'https://127.0.0.1:{self.server.port}/{url_below_host}'
        with urllib.request.urlopen(url, context=trusting_the_authority_alone, timeout=5) as response:
            return response.read()

    def test_a_recorded_url_is_answered_under_its_host_over_https_the_authority_vouches_for(self):
        self.assertEqual(self.get('api.open-meteo.com/v1/forecast?latitude=48.8530&longitude=2.3499'), b'{"current": {}}')
        self.assertEqual(self.server.asked, [FORECAST])

    def test_what_was_not_recorded_is_not_found(self):
        with self.assertRaises(urllib.error.HTTPError) as raised:
            self.get('api.open-meteo.com/v1/forecast?latitude=0')
        self.assertEqual(raised.exception.code, 404)


class Recordings(unittest.TestCase):
    def test_the_recordings_are_read_as_the_simulator_reads_them(self):
        web = recorded_web(WEB)
        self.assertTrue(any(url.startswith('https://api.open-meteo.com/v1/forecast?') for url in web))
        self.assertTrue(all(body for body in web.values()))


class Conf(unittest.TestCase):
    def test_the_conf_says_where_the_stubs_listen(self):
        self.assertEqual(e2e_conf('192.168.0.13', 8443, 8123), 'address = 192.168.0.13\nhttps_port = 8443\nntp_port = 8123\n')
