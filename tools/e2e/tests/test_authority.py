import unittest
from datetime import datetime, timezone

from cryptography import x509
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.x509.oid import ExtensionOID

from cute_display_e2e.authority import AUTHORITY, CERTIFICATE, KEY, issue_server_pem, make_authority_pem


class Authority(unittest.TestCase):
    def test_the_authority_signs_certificates_and_nothing_else(self):
        certificate = x509.load_pem_x509_certificate(make_authority_pem()[0])
        constraints = certificate.extensions.get_extension_for_oid(ExtensionOID.BASIC_CONSTRAINTS).value
        self.assertTrue(constraints.ca)
        self.assertEqual(constraints.path_length, 0)
        self.assertTrue(certificate.extensions.get_extension_for_oid(ExtensionOID.KEY_USAGE).value.key_cert_sign)

    def test_a_certificate_for_the_stubs_is_signed_by_the_authority_for_their_address(self):
        authority_pem, authority_key_pem = make_authority_pem()
        authority = x509.load_pem_x509_certificate(authority_pem)
        certificate = x509.load_pem_x509_certificate(issue_server_pem(authority_pem, authority_key_pem, '192.168.1.10')[0])
        certificate.verify_directly_issued_by(authority)
        names = certificate.extensions.get_extension_for_oid(ExtensionOID.SUBJECT_ALTERNATIVE_NAME).value
        self.assertEqual(names.get_values_for_type(x509.DNSName), ['192.168.1.10'])
        self.assertEqual([str(ip) for ip in names.get_values_for_type(x509.IPAddress)], ['192.168.1.10'])

    def test_the_certificates_hold_on_any_day_a_scenario_may_set(self):
        authority_pem, authority_key_pem = make_authority_pem()
        for pem in [authority_pem, issue_server_pem(authority_pem, authority_key_pem, '10.0.0.2')[0]]:
            certificate = x509.load_pem_x509_certificate(pem)
            self.assertLessEqual(certificate.not_valid_before_utc, datetime(2026, 1, 1, tzinfo=timezone.utc))
            self.assertGreaterEqual(certificate.not_valid_after_utc, datetime(2090, 1, 1, tzinfo=timezone.utc))

    def test_the_kept_authority_is_one_and_its_key_goes_with_it(self):
        certificate = x509.load_pem_x509_certificate((AUTHORITY / CERTIFICATE).read_bytes())
        key = serialization.load_pem_private_key((AUTHORITY / KEY).read_bytes(), password=None)
        self.assertIsInstance(key, ec.EllipticCurvePrivateKey)
        self.assertEqual(certificate.public_key(), key.public_key())
        self.assertTrue(certificate.extensions.get_extension_for_oid(ExtensionOID.BASIC_CONSTRAINTS).value.ca)
