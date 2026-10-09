"""The web that the test image asks for over HTTPS
(infrastructure::testing::to_local_stubs): the responses recorded in web/, each URL asked
under its host.

`python -m cute_display_e2e.stub_web_server` serves it by hand, with the time, and prints
the e2e.conf to put on the card."""
import http.server
import socket
import ssl
import sys
import tempfile
import threading
import time
from pathlib import Path

from .authority import AUTHORITY, CERTIFICATE, KEY, issue_server_pem
from .stub_time_server import StubTimeServer

WEB = Path(__file__).resolve().parents[2] / 'web'
HTTPS_PORT = 8443


def recorded_web(directory=WEB):
    """The URL of each recording, on its first line, and its body, after it: the simulator's
    `--web` format."""
    web = {}
    for recording in sorted(directory.glob('*.http')):
        url, _, body = recording.read_bytes().partition(b'\n')
        web[url.decode().rstrip()] = body
    return web


def local_address():
    """The address of this computer on its local network: the one it would send from to
    reach the Internet. Nothing is sent."""
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as probe:
        probe.connect(('192.0.2.1', 9))
        return probe.getsockname()[0]


def e2e_conf(address, https_port, ntp_port):
    return f'address = {address}\nhttps_port = {https_port}\nntp_port = {ntp_port}\n'


class StubWebServer:
    """With a certificate that the test authority signs for `address`. Port 0 lets the
    system choose; read the chosen port from `port`."""

    def __init__(self, address, *, port=HTTPS_PORT, web=None, authority=AUTHORITY):
        self.web = recorded_web() if web is None else web
        self.asked = []
        self._server = self._https_server(address, port, authority)
        self.port = self._server.server_address[1]
        threading.Thread(target=self._server.serve_forever, daemon=True).start()

    def close(self):
        self._server.shutdown()
        self._server.server_close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()

    def _https_server(self, address, port, authority):
        stub = self

        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                url = 'https://' + self.path.removeprefix('/')
                stub.asked.append(url)
                body = stub.web.get(url)
                self.send_response(404 if body is None else 200)
                self.send_header('Content-Length', str(len(body or b'')))
                self.end_headers()
                self.wfile.write(body or b'')

            def log_message(self, *_):
                pass

        server = http.server.ThreadingHTTPServer(('', port), Handler)
        certificate, key = issue_server_pem((authority / CERTIFICATE).read_bytes(), (authority / KEY).read_bytes(), address)
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        # ssl reads a certificate chain only from files.
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / 'server.pem').write_bytes(certificate)
            (Path(directory) / 'server.key').write_bytes(key)
            context.load_cert_chain(Path(directory) / 'server.pem', Path(directory) / 'server.key')
        server.socket = context.wrap_socket(server.socket, server_side=True)
        return server


def main():
    address = local_address()
    with StubWebServer(address) as web, StubTimeServer() as time_server:
        print(f'Serving {len(web.web)} recordings and the time on {address}. Put on the card, as '
              'cute-display-e2e/e2e.conf:\n')
        print(e2e_conf(address, web.port, time_server.port))
        print('Ctrl-C stops.')
        seen = 0
        try:
            while True:
                time.sleep(0.5)
                for url in web.asked[seen:]:
                    print(f'asked {url}: {"answered" if url in web.web else "not recorded"}')
                seen = len(web.asked)
        except KeyboardInterrupt:
            return 0


if __name__ == '__main__':
    sys.exit(main())
