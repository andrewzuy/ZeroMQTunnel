"""pytest configuration for ZeroMQ Tunnel tests."""
import sys
import logging
import pytest

# Configure logging to capture stdout/stderr from background tasks in pytest-asyncio
logging.basicConfig(
    level=logging.DEBUG,
    format='%(levelname)s [%(name)s]: %(message)s',
    stream=sys.stdout,
    force=True,  # Force reconfiguration even if already configured
)

# Enable logging for zmq and asyncio debug output
logging.getLogger("zmq").setLevel(logging.DEBUG)
logging.getLogger("asyncio").setLevel(logging.DEBUG)


def pytest_configure(config):
    """Configure logging for pytest session."""
    handler = logging.StreamHandler(sys.stdout)
    handler.setLevel(logging.DEBUG)

    # Create a formatter with timestamp and message
    formatter = logging.Formatter('%(levelname)s: %(name)s: %(message)s')
    handler.setFormatter(formatter)

    logger = logging.getLogger("zeromqtunnel")
    logger.addHandler(handler)
    logger.setLevel(logging.DEBUG)

    # Capture output from zmq and asyncio modules
    for name in ["zmq", "asyncio", "zeromqtunnel.server", "zeromqtunnel.client", "zeromqtunnel.protocol"]:
        logging.getLogger(name).setLevel(logging.DEBUG)


def log_cli(record_warnings=False, disable_pretty=False):
    # Enable verbose output
    import traceback
