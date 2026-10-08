import socket
import struct
import unittest
from datetime import datetime, timezone

from cute_display_e2e.stub_time_server import (NTP_PACKET_BYTES, NTP_TO_UNIX_SECONDS, NTP_TRANSMIT_TIMESTAMP, FakeRunningTime,
                                               StubTimeServer, ntp_answer)

SET_AT = int(datetime(2026, 10, 1, 12, 12, tzinfo=timezone.utc).timestamp())


def ask(port):
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as client:
        client.settimeout(5)
        client.sendto(bytes(NTP_PACKET_BYTES), ('127.0.0.1', port))
        answer, _ = client.recvfrom(NTP_PACKET_BYTES * 2)
    return answer


class TimeServer(unittest.TestCase):
    def test_the_time_is_the_one_set_running_on(self):
        with StubTimeServer(port=0, running_time=FakeRunningTime(SET_AT)) as server:
            answer = ask(server.port)
        self.assertEqual(answer[0] & 0b111, 4, 'a server')
        seconds = struct.unpack_from('>I', answer, NTP_TRANSMIT_TIMESTAMP)[0] - NTP_TO_UNIX_SECONDS
        self.assertIn(seconds, range(SET_AT, SET_AT + 3))

    def test_its_port_is_free_again_once_closed(self):
        with StubTimeServer(port=0) as server:
            port = server.port
        with StubTimeServer(port=port) as again:
            self.assertEqual(len(ask(again.port)), NTP_PACKET_BYTES)

    def test_an_ntp_answer_carries_the_second_it_is_made_for(self):
        self.assertEqual(struct.unpack_from('>I', ntp_answer(SET_AT), NTP_TRANSMIT_TIMESTAMP)[0], SET_AT + NTP_TO_UNIX_SECONDS)
