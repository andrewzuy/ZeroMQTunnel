"""Entry point for running zmqtunnel as a module."""

import sys
import asyncio
from zmqtunnel.cli import main

async def _run(args=None):
    return await main(args)

if __name__ == "__main__":
    exit_code = asyncio.run(_run())
    sys.exit(exit_code)
