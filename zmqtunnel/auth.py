"""
ZAP (ZeroMQ Authentication Protocol) authenticator for server.
"""

import os
from pathlib import Path


class ThreadAuthenticator:
    """
    Authenticator that allows specific clients to connect based on their public key.
    Uses ZAP protocol for application-layer authentication in addition to Curve transport.
    """

    def __init__(self, allowed_keys_dir: Path):
        self.keys_dir = allowed_keys_dir
        # Load known client public keys into a set
        self._known_clients = self._load_client_keys()

    def _load_client_keys(self) -> set[str]:
        """Load all .key files from the keys directory."""
        if not self.keys_dir.exists():
            return set()

        clients = set()
        for key_file in self.keys_dir.glob("*_public.key"):
            try:
                pub_key = key_file.read_text().strip()
                # Only register clients whose keys have been explicitly generated
                secret_file = self.keys_dir / f"{key_file.stem}_secret.key"
                if secret_file.exists():
                    clients.add(pub_key)
            except Exception as e:
                print(f"Error loading key {key_file}: {e}")

        return clients

    def is_allowed(self, client_public_key: str) -> bool:
        """Check if a client's public key is allowed to connect."""
        return client_public_key in self._known_clients
