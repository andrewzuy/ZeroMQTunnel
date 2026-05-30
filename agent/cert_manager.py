import os
import base64
import zmq


def get_cert_path(keydir='/tmp/.curve'):
    """Get paths for agent certificate files."""
    priv_file = os.path.join(keydir, 'agent.private.key')
    pub_file = os.path.join(keydir, 'agent.public.key')
    
    if not (os.path.exists(priv_file) and os.path.exists(pub_file)):
        from pyzmq import Context, Socket
        
        print("Generating new agent CURVE keypair...")
        ctx = Context()
        socket = zmq.Socket(ctx, zmq.PAIR)
        
        priv_content = socket.getsockopt_string(zmq.CURVE_SECRETKEY).encode()
        pub_content = socket.getsockopt_string(zmq.CURVE_PUBLICKEY).encode()
        
        with open(priv_file, 'wb') as f:
            f.write(base64.b64decode(priv_content))
            
        with open(pub_file, 'wb') as f:
            f.write(base64.b64decode(pub_content))
        
        socket.close()
        ctx.term()
    
    return priv_file, pub_file

