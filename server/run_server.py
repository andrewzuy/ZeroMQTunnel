
#!/usr/bin/env python3
"""PHASE 3: Complete self-healing server with heartbeat & cleanup."""

import zmq
import time
from datetime import datetime


class _Config:
    RECONNECT_IVL = 1000       # ms initial reconnect delay  
    RECONNECT_IVL_MAX = 30000  # max reconnect delay (ms)
    HEARTBEAT_INTERVAL = 5.0   # seconds between heartbeats
    TIMEOUT_THRESHOLD = 15      # missed heartbeat threshold  
    CLEANUP_TIMEOUT = 30        # stale agent timeout


class ServiceAgent:
    """Complete server with Heartbeat + Self-Healing (Phase 3)."""
    
    def __init__(self):
        self.ctx = zmq.Context()
        
        # Control socket for agent management
        self.control_socket = self.ctx.socket(zmq.ROUTER)
        
        keyfile = '/tmp/.server/server.key'  
        curv_key = b''
        
        if os.path.exists(keyfile):
            try:
                with open(keyfile) as f:
                    content = f.read()
                    if isinstance(content, str) and content.strip():
                        curv_key = content.encode('utf-8') or b''
            except Exception as e:
                print(f"Error loading key: {e}")
        
        self.control_socket.setsockopt_string(
            zmq.CURVE_SERVERKEY, 
            curv_key.decode() if isinstance(curv_key, bytes) else (curv_key or b'').decode() or b''.decode()
        )
        self.control_socket.bind(f"tcp://*:5555")
        
        # Data plane sockets
        self.stream_socket = self.ctx.socket(zmq.STREAM)  
        self.stream_socket.bind(f"0.0.0.0:443")
        
        self.router_socket = self.ctx.socket(zmq.ROUTER)  
        self.router_socket.setsockopt_string(
            zmq.IDENTITY, "service_data".encode()
        )
        self.router_socket.bind(f"tcp://*:5556")
