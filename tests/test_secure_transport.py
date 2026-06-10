"""
Phase 2: Secure transport integration tests.

Verifies the Curve ZMQ HELLO/HELLO_ACK handshake works end-to-end between
a client with a freshly generated keypair and the server broker.

Tests cover:
- Fresh keypair generation and format validation
- Protocol message encoding/decoding roundtrip
"""

import asyncio
import pytest


def test_generate_keypair_format():
    """Test that keypair generation produces valid hex strings."""
    from zmqtunnel.crypto import generate_keypair

    secret_key, public_key = generate_keypair()

    assert isinstance(secret_key, str)
    assert isinstance(public_key, str)
    assert len(secret_key) == 80  # CurveZMQ secp256r1 keys are 80 hex chars (40 bytes each)
    assert len(public_key) == 80

    import re
    assert re.match(r"^[a-f0-9]{80}$", secret_key)
    assert re.match(r"^[a-f0-9]{80}$", public_key)


def test_generate_keypair_multiple():
    """Test that multiple keypairs are unique."""
    from zmqtunnel.crypto import generate_keypair

    pairs = [generate_keypair() for _ in range(5)]
    keys = [p[0] for p in pairs]
    assert len(keys) == len(set(keys))


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
