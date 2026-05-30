# ZeroMQ Tunnel Project Setup

## Quick Install Options

### Option 1: Debian/Ubuntu System Packages (Recommended)
```bash
apt-get update && apt-get install -y python3-pyzmq libsodium-dev libzmq3-dev
source venv/bin/activate
pip install pyzmq==26.0.3 aiofiles numpy
```

### Option 2: Using pip with system bindings (Python 3.14+)
```bash
python3 -m venv venv && source venv/bin/activate
apt-get install -y libsodium-dev libzmq3-dev
pip install pyzmq --no-cache-dir
pip install aiofiles numpy
```

### Option 3: Full pip installation (standalone, ~50MB)
```bash
python3 -m venv venv && source venv/bin/activate
pip install pyzmq==26.0.3 aiofiles numpy
# This bundles its own libzmq libsodium if system libs aren't found
```

## Dependencies

- `pyzmq` - ZeroMQ Python bindings (required)
- `aiofiles` - Async file handling (optional)  
- `numpy` - Optional: for large message buffers

## Quick Start After Setup

```bash
cd ZeroMQTunnel

# Generate certificates
python server/cert_manager.py

# Mock service on localhost:443
python server/mock_http_server.py &

# Start agent
python agent/example.py

# Start tunnel server
python server/main.py
```

## Test

```bash
curl http://localhost/
# Should return: "Hello from localhost\n"

# Kill services (Ctrl+C)
```

EOF
cat /home/andrew/Development/ZeroMQTunnel/SETUP.md
