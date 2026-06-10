"""
Curve ZMQ encryption and key management.
"""

import os
from pathlib import Path
from typing import Tuple
import zmq


def generate_keypair() -> Tuple[str, str]:
    """Generate a new Curve ZMQ keypair.

    Returns:
        Tuple of (private_key_hex_string, public_key_hex_string).
        Uses secp256k1 curve with 40-character hex strings (256-bit keys).
    """
    # Use zmq's built-in CurveZMQ keypair generation
    secret_bytes, public_bytes = zmq.curve_keypair()
    return secret_bytes.hex(), public_bytes.hex()


def load_keyfile(path: Path) -> str:
    """Load a key from file.

    Args:
        path: Path to the key file.

    Returns:
        Key string (may be hex or raw bytes encoded).
    """
    content = path.read_text().strip()
    # Try as hex first, then try as raw if it looks like a key
    try:
        return bytes.fromhex(content).hex()
    except ValueError:
        # It might already be in the right format
        return content


def set_curve_options(
    sock: zmq.Socket,
    public_key: str,
    secret_key: str | None = None,
) -> None:
    """Configure Curve ZMQ options on a socket.

    Args:
        sock: Socket to configure.
        public_key: Server's public key (required for all Curve sockets).
        secret_key: Client's secret key for authentication.
    """
    if secret_key:
        sock.setsockopt(zmq.CURVE_SECRETKEY, bytes.fromhex(secret_key))
    sock.setsockopt(zmq.CURVE_SERVER, 0)
    sock.setsockopt(zmq.CURVE_PUBLICKEY, bytes.fromhex(public_key))

    # Heartbeat settings for reliability
    sock.setsockopt(zmq.HEARTBEAT_IVL, 10_000)  # 10 seconds
    sock.setsockopt(zmq.HEARTBEAT_TIMEOUT, 60_000)  # 60 seconds
