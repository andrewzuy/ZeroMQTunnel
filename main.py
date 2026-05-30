#!/usr/bin/env python3
"""Main entry point for the tunnel server."""

import zmq


def generate_curve_certificates(keydir='.', host_id=None):
    """Generate or load CURVE keypairs for server/auth."""
    
    import os, base64
    
    if not keydir:
        return "/tmp/.curve"
    
    keydir = f".{keydir}"
    os.makedirs(keydir, exist_ok=True)
    
    # Load first available keypair
    priv_file = f"{keydir}/server.key"
    pub_file = f"{keydir}/server.pub"
    
    if not (os.path.exists(priv_file) and os.path.exists(pub_file)):
        print("Generating new CURVE keypairs...")
        
        ctx = zmq.Context()
        socket = zmq.Socket(ctx, zmq.PAIR)
        priv_content = socket.getsockopt_string(zmq.CURVE_SECRETKEY).encode()
        pub_content = socket.getsockopt_string(zmq.CURVE_PUBLICKEY).encode()
        
        with open(priv_file, 'wb') as f:
            f.write(base64.b64decode(priv_content))
            
        with open(pub_file, 'wb') as f:
            f.write(base64.b64decode(pub_content))
        
        socket.close()
        ctx.term()
    
    print(f"Cert directory: {keydir}")
    return keydir


if __name__ == '__main__':
    print("Tunnel Server v1.0")
    print("Initializing components...")
    cert_dir = generate_curve_certificates('/tmp/.server')
