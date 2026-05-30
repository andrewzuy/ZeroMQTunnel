#!/usr/bin/env python3
"""Socket bindings for server data plane."""

import zmq


class DataPlaneBinder:
    """Handles ZMQ_STREAM and DEALER binding configuration."""
    
    def __init__(self, ctx):
        self.ctx = ctx
        self.stream_socket: zmq.Socket = None
        self.router_socket: zmq.Socket = None
        
    def bind_streams(self, exposed_port=1443, data_port=5556):
        """Bind both external listener and internal router."""
        
        # External TCP stream listener (plain TCP, no CURVE)
        self.stream_socket = self.ctx.socket(zmq.STREAM)  
        self.stream_socket.bind(f"tcp://0.0.0.0:{exposed_port}")
        
        # Internal data relay router (with CURVE auth)
        self.router_socket = self.ctx.socket(zmq.ROUTER)
        self.router_socket.setsockopt(zmq.IDENTITY, b"tunnel_data")
        self.router_socket.bind(f"tcp://*:data_port")
        
        print(f"Bound: STREAM:{exposed_port}, ROUTER:{data_port}")
    
    def bind_client(self, data_address):
        """Bind the client-facing socket."""
        if not hasattr(self.ctx, 'client_socket'):
            self.client_socket = self.ctx.socket(zmq.ROUTER)
        
        self.client_socket.bind(data_address)
        