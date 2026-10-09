"""The certificate authority that the test image trusts, created once and kept in
device/ca/, and the certificates it signs for the stubs, at the address of the computer they
run on.

`python -m cute_display_e2e.authority` creates it. If you create it again, build and flash
the test image again, because the image contains its certificate."""
import ipaddress
import sys
from datetime import datetime, timezone
from pathlib import Path

from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.x509.oid import NameOID

AUTHORITY = Path(__file__).resolve().parents[2] / 'device' / 'ca'
CERTIFICATE = 'ca.pem'
KEY = 'ca.key'
# A scenario sets the clock to its own day, which may be before today: the certificates are
# valid from long before any scenario to long after.
VALID_FROM = datetime(2020, 1, 1, tzinfo=timezone.utc)
VALID_UNTIL = datetime(2099, 12, 31, tzinfo=timezone.utc)


def _certificate(subject, issuer, public_key, signing_key, extensions):
    builder = (x509.CertificateBuilder()
               .subject_name(x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, subject)]))
               .issuer_name(x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, issuer)]))
               .public_key(public_key)
               .serial_number(x509.random_serial_number())
               .not_valid_before(VALID_FROM)
               .not_valid_after(VALID_UNTIL))
    for extension, critical in extensions:
        builder = builder.add_extension(extension, critical=critical)
    return builder.sign(signing_key, hashes.SHA256())


def _pem(certificate, key):
    return (certificate.public_bytes(serialization.Encoding.PEM),
            key.private_bytes(serialization.Encoding.PEM, serialization.PrivateFormat.PKCS8, serialization.NoEncryption()))


def make_authority_pem():
    key = ec.generate_private_key(ec.SECP256R1())
    name = 'Cute Display test image authority'
    certificate = _certificate(name, name, key.public_key(), key, [
        (x509.BasicConstraints(ca=True, path_length=0), True),
        (x509.KeyUsage(digital_signature=False, content_commitment=False, key_encipherment=False, data_encipherment=False,
                       key_agreement=False, key_cert_sign=True, crl_sign=True, encipher_only=False, decipher_only=False), True),
        (x509.SubjectKeyIdentifier.from_public_key(key.public_key()), False),
    ])
    return _pem(certificate, key)


def issue_server_pem(authority_pem, authority_key_pem, ip_address):
    """mbedTLS compares the URL's host with the names as text, so the address is also given
    as a name."""
    authority = x509.load_pem_x509_certificate(authority_pem)
    authority_key = serialization.load_pem_private_key(authority_key_pem, password=None)
    key = ec.generate_private_key(ec.SECP256R1())
    authority_name = authority.subject.get_attributes_for_oid(NameOID.COMMON_NAME)[0].value
    certificate = _certificate(ip_address, authority_name, key.public_key(), authority_key, [
        (x509.SubjectAlternativeName([x509.DNSName(ip_address), x509.IPAddress(ipaddress.ip_address(ip_address))]), False),
        (x509.BasicConstraints(ca=False, path_length=None), True),
        (x509.ExtendedKeyUsage([x509.oid.ExtendedKeyUsageOID.SERVER_AUTH]), False),
        (x509.AuthorityKeyIdentifier.from_issuer_public_key(authority.public_key()), False),
    ])
    return _pem(certificate, key)


def main(arguments):
    replacing = arguments == ['--replace']
    if (AUTHORITY / CERTIFICATE).exists() and not replacing:
        print(f'{AUTHORITY} already holds the authority the test image trusts. --replace makes another, '
              'and the test image must then be built and flashed again.', file=sys.stderr)
        return 1
    certificate, key = make_authority_pem()
    AUTHORITY.mkdir(parents=True, exist_ok=True)
    (AUTHORITY / CERTIFICATE).write_bytes(certificate)
    (AUTHORITY / KEY).write_bytes(key)
    print(f'{AUTHORITY / CERTIFICATE}: build and flash the test image, which carries it')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
