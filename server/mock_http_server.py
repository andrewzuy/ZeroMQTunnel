#!/usr/bin/env python3
"""Simple HTTP server running on localhost:443 for tunnel testing."""

import http.server
import socketserver


class CustomHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.end_headers()
        self.wfile.write(f"Hello from {self.path}\n".encode())
    
    def log_message(self, format, *args):
        print(f"[MOCK HTTP] {args[0]}")


PORT = 443

with socketserver.TCPServer(("127.0.0.1", PORT), CustomHandler) as httpd:
    print(f"Mock HTTP server running on http://127.0.0.1:{PORT}")
    try:
        httpd.handle_request()
        httpd.handle_request()  # Handle two connections for demo
    except KeyboardInterrupt:
        pass
