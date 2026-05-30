#!/usr/bin/env python3
"""Service agent with complete data plane."""

import zmq
import json
import socket
import select


class ServiceAgent:
    """Full service-side tunnel implementation."""
    
    SERVER_CONTROL = "tcp://[::1]:5555"  
    DATA_PORT_OFFSET = 100
    
    def __init__(self, local_port=443):
        self.ctx = zmq.Context()
        
        # Control socket to server
        self.control_socket = self.ctx.socket(zmq.DEALER)  
        self.control_socket.connect(self.SERVER_CONTROL)
        
        # Data relay socket  
        self.data_socket = self.ctx.socket(zmq.DEALER)
        self.data_socket.setsockopt_string(
            zmq.CURVE_SERVERKEY, 
            open('/tmp/.curve/server.key').read()
        )
        self.data_socket.connect(f"tcp://[::1]:5556")
        
        # Track local service and sessions
        self.local_port = local_port
        self.tcp_pool: dict = {}  # session_id -> tcp_socket
        
        print("Agent control + data sockets initialized")

    def register_service(self, host='127.0.0.1'):
        """Register this agent with the server."""
        
        message = {
            'type': 'REGISTER_FORWARD',
            'service_id': 'svc443',
            'local_host': host,
            'local_port': str(self.local_port),
            'agent_id': str(os.getpid())  
        }
        
        framing = [self.control_socket.identity]  
        
        # Send multipart message
        try:
            self.control_socket.send(json.dumps(message).encode())
            
            # Wait for response
            _ = self.control_socket.recv()
            
            print(f"Registered service on {host}:{self.local_port}")
            return True
            
        except Exception as e:
            print(f"Registration failed: {e}")
            return False
