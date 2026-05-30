#!/usr/bin/env python3
"""Example of connecting a service agent to the tunnel server."""

import zmq
import sys
import json


def register_agent(agent, control_address):
    """Register this agent with the server."""
    
    message = {
        'type': 'REGISTER_FORWARD',
        'service_id': 'default',
        'local_host': '127.0.0.1',
        'local_port': 443,
    }
    
    print(f"Connecting agent to server at {control_address}")
    socket.connect(control_address)
    response = socket.recv()
    print(f"Registration response: {response}")


if __name__ == '__main__':
    import zmq
    
    ctx = zmq.Context()
    socket = ctx.socket(zmq.DEALER)
    
    register_agent(socket, "tcp://[::1]:5555")
