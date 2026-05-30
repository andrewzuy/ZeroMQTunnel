#!/bin/bash
set -e

echo "╔══════════════════════════════════════════════════╗"
echo "║   ZeroMQ Tunnel - CURVE Certificate Setup         ║"
echo "╚══════════════════════════════════════════════════╝"

venv="/home/andrew/Development/ZeroMQTunnel/venv"
source "$venv/bin/activate"

# Install deps  
pip install pyzmq aiofiles numpy > /dev/null 2>&1

python3 << 'PYEOF'
import zmq, os, base64

cert_dir = '/tmp/.server_curves'
os.makedirs(cert_dir, exist_ok=True)

ctx = zmq.Context()
socket = zmq.Socket(ctx, zmq.PAIR)

# Generate CURVE certificate for ZAP auth
print("Generating CURVE certificate...")
priv_content = socket.getsockopt_string(zmq.CURVE_SECRETKEY).encode()
pub_content = socket.getsockopt_string(zmq.CURVE_PUBLICKEY).encode()

with open(f'{cert_dir}/server.key', 'wb') as f:
    f.write(base64.b64decode(priv_content))
    
with open(f'{cert_dir}/server.pub', 'wb') as f:
    f.write(base64.b64decode(pub_content))
    
socket.close()
ctx.term()

print('Certificate path:', cert_dir)  
print("  - server.key (private)")
print("  - server.pub  (public)")
PYEOF

echo "✓ Setup complete. Running mock service + tunnel test..."  

# Start simple HTTP mock on localhost:443 for testing
cd "$venv" && python3 -c "
from http.server import HTTPServer, BaseHTTPRequestHandler

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-type', 'text/html')
        self.end_headers()
        self.wfile.write(b'<html><body><h1>ZeroMQ Tunnel Test OK</h1></body></html>')
        self.wfile.write(f'<p>Port: 443</p>'.encode())
    def log_message(self, format, *args):
        pass

print('Starting mock HTTP server on localhost:443...')    
HTTPServer(('127.0.0.1', 443), Handler).handle_request()
" &

echo "Mock service started (Ctrl+C to stop)"  

exit 0
