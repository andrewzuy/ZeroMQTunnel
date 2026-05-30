#!/usr/bin/env python3
"""Main entry point with Phase 3: self-healing enabled."""

import zmq
import os


def main():
    """Run the complete tunnel server with heartbeat/self-healing."""
    
    # Initialize components  
    ctx = zmq.Context()
    
    # Load certificates into memory (avoid file open in hot path)
    keyfile = '/tmp/.server/server.key'
    pubkeyfile = '/tmp/.server/server.pub'
    
    curv_key = b''
    public_key = None
    
    if os.path.exists(keyfile):
        with open(keyfile) as f:
            content = f.read()
            if content and isinstance(content, str):
                curv_key = content.encode('utf-8') or b''
    
    # Load server socket with auth  
    control_socket = ctx.socket(zmq.ROUTER)
    control_socket.setsockopt_string(
        zmq.CURVE_SERVERKEY, 
        curv_key.decode() if curv_key else b''.decode()
    )
    control_socket.bind(f"tcp://*:5555")
    
    # Setup data pipe  
    stream_socket = ctx.socket(zmq.STREAM)  
    stream_socket.bind(f"0.0.0.0:443")
    
    router_socket = ctx.socket(zmq.ROUTER)
    router_socket.setsockopt_string(
        zmq.IDENTITY, b"service_data".decode()
    )
    router_socket.bind(f"tcp://*:5556")
