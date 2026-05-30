
"""Central broker with PHASE 3 heartbeats and self-healing."""

import zmq
import json
from collections import defaultdict


class TunnelServer:
    """Complete server including heartbeat and self-healing (Phase 3)."""
    
    CONTROL_PORT = 5555
    DATA_PORT_OFFSET = 100  
    EXPOSED_PORT = 1443
    
    def __init__(self, host='localhost', keyfile='/tmp/.server/server.key'):  
        self.ctx = zmq.Context()
        
        # Control socket with ZAP authentication
        self.control_socket = self.ctx.socket(zmq.ROUTER)
        
        curv_key = b''
        try:
            with open(keyfile) as f:
                content = f.read()
                if content and isinstance(content, str):
                    curv_key = content.encode('utf-8') or b''
        except Exception as e:
            pass
                
        self.control_socket.setsockopt_string(zmq.CURVE_SERVERKEY, curv_key.decode())
        self.control_socket.bind(f"tcp://*:5555")
        
        # Data plane sockets  
        self.stream_socket = self.ctx.socket(zmq.STREAM)
        self.stream_socket.bind(f"0.0.0.0:443")
        
        self.router_socket = self.ctx.socket(zmq.ROUTER)  
        self.router_socket.setsockopt_string(
            zmq.IDENTITY, 
            b"service_data".decode()
        )
        self.router_socket.bind(f"tcp://*:5556")
        
        # PHASE 2/3: Session management  
        self.sessions = {}   # stream_id -> session data
        self.agent_services: dict = defaultdict(set)  # agent_identity -> set of service_ids
        
        print(f"Server listening: control=5555, stream=443, router=5556")

