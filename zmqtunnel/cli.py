"""
CLI entry point for zmqtunnel.
"""

import asyncio
import argparse
import sys
from pathlib import Path
from typing import Literal


def get_default_config_path() -> Path:
    """Get default config path in user's home."""
    return Path("~/.zmqtunnel/config.yaml").expanduser()


def get_default_keys_path() -> Path:
    """Get default keys directory path."""
    return Path("~/.zmqtunnel/keys").expanduser()


async def server_main() -> int:
    """Run the ZMQ tunnel server (broker)."""
    from zmqtunnel.server.broker import ServerBroker

    config_path = argparse.Namespace(
        server_key=get_default_config_path(),
        keys_dir=get_default_keys_path(),
        bind_addr="tcp://*:5555",
    )

    broker = ServerBroker(config=config_path)
    await broker.run()
    return 0


async def local_main(local_bind: str, remote_target: str) -> int:
    """Run in local forwarding mode (ssh -L equivalent)."""
    from zmqtunnel.client.agent import ClientAgent, ClientConfig

    config = ClientConfig(
        server_addr="tcp://localhost:5555",
        server_key=get_default_config_path(),
        client_key=f"{get_default_keys_path()}/client_secret.key",
        local_bind=local_bind,
    )

    agent = ClientAgent(config=config)
    await agent.run()
    return 0


async def remote_main(remote_bind: str, local_target: str) -> int:
    """Run in remote forwarding mode (ssh -R equivalent)."""
    from zmqtunnel.client.agent import ClientAgent, ClientConfig

    config = ClientConfig(
        server_addr="tcp://localhost:5555",
        server_key=get_default_config_path(),
        client_key=f"{get_default_keys_path()}/client_secret.key",
        remote_bind=remote_bind,
    )

    agent = ClientAgent(config=config)
    await agent.run()
    return 0


async def keygen_main(output: Path | None = None) -> int:
    """Generate a new Curve ZMQ keypair."""
    from zmqtunnel.crypto import generate_keypair

    pair, public_key = generate_keypair()

    if output is not None:
        private_path = output.with_name(f"{output.name}_secret.key")
        public_path = output.with_name(f"{output.name}_public.key")

        with open(private_path, "w") as f:
            f.write(pair[0])
        with open(public_path, "w") as f:
            f.write(public_key)

    return 0


def create_parser() -> argparse.ArgumentParser:
    """Create and configure the argument parser."""
    parser = argparse.ArgumentParser(
        prog="zmqtunnel",
        description="SSH-like local and remote port forwarding over CurveZMQ tunnels"
    )

    subparsers = parser.add_subparsers(dest="command", help="Available commands")

    # Server command
    server_parser = subparsers.add_parser("server", help="Run as server/broker")
    server_parser.set_defaults(func=server_main)
    server_parser.add_argument(
        "--bind-addr",
        default="tcp://*:5555",
        help="Address to bind the server listener"
    )

    # Local forward command (-L equivalent)
    local_parser = subparsers.add_parser("local", help="Local forwarding mode (ssh -L)")
    local_parser.set_defaults(func=local_main)
    local_parser.add_argument("-L", required=True, dest="local_bind", help="Local bind address:target:port")

    # Remote forward command (-R equivalent)
    remote_parser = subparsers.add_parser("remote", help="Remote forwarding mode (ssh -R)")
    remote_parser.set_defaults(func=remote_main)
    remote_parser.add_argument("-R", required=True, dest="remote_bind", help="Remote bind address:target:port")

    # Key generation command
    keygen_parser = subparsers.add_parser("keygen", help="Generate a new Curve ZMQ keypair")
    keygen_parser.set_defaults(func=keygen_main)
    keygen_parser.add_argument("-o", "--output", type=Path, help="Output path for keypair files")

    return parser


async def main(args: list[str] | None = None) -> int:
    """Main entry point."""
    parser = create_parser()
    parsed_args = parser.parse_args(args)

    if not parsed_args.command:
        parser.print_help()
        return 1

    try:
        return await parsed_args.func()
    except KeyboardInterrupt:
        print("\nShutting down...")
        return 0
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    asyncio.run(main())
