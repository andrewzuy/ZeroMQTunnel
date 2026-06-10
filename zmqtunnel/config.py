"""
Configuration and key loading.
"""

from dataclasses import dataclass
from pathlib import Path
from typing import Optional
import yaml


@dataclass
class ServerConfig:
    """Server (broker) configuration."""
    bind_addr: str = "tcp://*:5555"
    keys_dir: Path = Path("~/.zmqtunnel/keys").expanduser()


@dataclass
class ClientConfig:
    """Client configuration for local or remote forwarding."""
    server_addr: str = "tcp://localhost:5555"
    server_key_path: Path = Path("~/.zmqtunnel/server_public.key").expanduser()
    client_key_path: Path = Path("~/.zmqtunnel/client_secret.key").expanduser()

    # For local forwarding (-L)
    local_bind: Optional[str] = None

    # For remote forwarding (-R)
    remote_bind: Optional[str] = None


def load_config(path: Optional[Path] = None) -> ClientConfig:
    """Load configuration from YAML file.

    Args:
        path: Path to config file. Uses ~/.zmqtunnel/config.yaml by default.

    Returns:
        ClientConfig instance.

    Raises:
        FileNotFoundError: If config file doesn't exist.
        yaml.YAMLError: If config is invalid YAML.
    """
    if path is None:
        path = Path("~/.zmqtunnel/config.yaml").expanduser()

    if not path.exists():
        raise FileNotFoundError(f"Config file not found: {path}")

    with open(path, "r") as f:
        data = yaml.safe_load(f) or {}

    return ClientConfig(
        server_addr=data.get("server_addr", "tcp://localhost:5555"),
        server_key_path=Path(data.get("server_key", "~/.zmqtunnel/server_public.key")).expanduser(),
        client_key_path=Path(data.get("client_secret", "~/.zmqtunnel/client_secret.key")).expanduser(),
        local_bind=data.get("local_bind"),
        remote_bind=data.get("remote_bind"),
    )


def ensure_keys_exist() -> tuple[Path, Path]:
    """Ensure key files exist, creating them if necessary.

    Returns:
        Tuple of (client_private_key_path, client_public_key_path).
    """
    keys_dir = Path("~/.zmqtunnel/keys").expanduser()
    keys_dir.mkdir(parents=True, exist_ok=True)

    private_key = keys_dir / "client_secret.key"
    public_key = keys_dir / "client_public.key"

    if not private_key.exists():
        from zmqtunnel.crypto import generate_keypair
        pair, pub_key = generate_keypair()
        private_key.write_text(pair[0])
        public_key.write_text(pub_key)

    return private_key, public_key


def ensure_server_key_exists(server_key_path: Path) -> str:
    """Ensure server public key exists.

    Args:
        server_key_path: Path to the server's public key file.

    Returns:
        Server public key string.

    Raises:
        FileNotFoundError: If server key file doesn't exist.
    """
    if not server_key_path.exists():
        raise FileNotFoundError(f"Server public key not found: {server_key_path}")

    return server_key_path.read_text().strip()
