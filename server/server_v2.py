import zmq
from .bindings import DataPlaneBinder


class TunnelServer:
    """Server with enhanced data plane binding."""
    
    CONTROL_PORT = 5555
    DATA_PORT_OFFSET = 100
    EXPOSED_PORT = 1443
    
    def __init__(self, server_keyfile, server_pubkey):
        self.ctx = zmq.Context()
        
        # Control socket with ZAP authentication
        self.control_socket = self.ctx.socket(zmq.ROUTER)
        self.control_socket.setsockopt_string(
            zmq.CURVE_SERVERKEY, 
            open(server_keyfile).read()
        )
        self.control_socket.bind(f"tcp://*:5555")
        
        # Data plane binding  
        binder = DataPlaneBinder(self.ctx)
        binder.bind_streams(exposed_port=1443, data_port=5556)
        
        self.stream_socket = binder.stream_socket
        self.router_socket = binder.router_socket
        
        # Session tracking
        self.sessions: dict = {}
        self.agent_services: dict = dict()
        
        print(f"Server initialized on ports {self.CONTROL_PORT}, 5556, {self.EXPOSED_PORT}")
