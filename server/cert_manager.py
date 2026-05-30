import os
import json
import base64


def generate_curve_keypair():
    from pyzmq import CURVE_KEYPAIR_DIR

    keydir = CURVE_KEYPAIR_DIR
    os.makedirs(keydir, exist_ok=True)
    
    priv_file = os.path.join(keydir, 'server_private.key')
    pub_file = os.path.join(keydir, 'server_public.key')
    
    if not (os.path.exists(priv_file) and os.path.exists(pub_file)):
        from pyzmq.eventloop.zmqstream import ZMQStream
        
        print("Generating new CURVE keypairs...")
        priv_content, pub_content = None, None
        
        # Generate using zmq's built-in mechanism
        ctx = zmq.Context()
        socket = zmq.Socket(ctx, zmq.PAIR)
        
        socket.setsockopt(zmq.CURVE_PUBLICKEY, b'')
        socket.setsockopt(zmq.CURVE_SECRETKEY, b'')
        
        priv_content = socket.getsockopt_string(zmq.CURVE_SECRETKEY).encode()
        pub_content = socket.getsockopt_string(zmq.CURVE_PUBLICKEY).encode()
        
        socket.close()
        ctx.term()
        
        with open(priv_file, 'wb') as f:
            f.write(base64.b64decode(priv_content))
            
        with open(pub_file, 'wb') as f:
            f.write(base64.b64decode(pub_content))
    
            print("Certificate saved to", keydir)
    return priv_file, pub_file


if __name__ == '__main__':
    generate_curve_keypair()
EOF