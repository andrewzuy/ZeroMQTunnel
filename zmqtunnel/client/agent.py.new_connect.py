    def connect_and_authenticate(self) -> None:
        """Initialize ZMQ connection to server and complete handshake (blocking mode for Python 3.14)."""
        from zmqtunnel.protocol import PROTOCOL_VERSION, MSG_TYPES

        # Use DEALER socket with ROUTER pattern - this works correctly with plain TCP in most Python builds
        self.dealer = self.ctx.socket(zmq.DEALER)
        print("[CLIENT] Using DEALER socket (plain TCP)", flush=True, file=sys.stderr)

        # Reconnection settings for DEALER sockets
        self.dealer.setsockopt(zmq.RECONNECT_IVL, 100)
        self.dealer.setsockopt(zmq.RECONNECT_IVL_MAX, 30000)
        self.dealer.setsockopt(zmq.HEARTBEAT_IVL, 10000)
        self.dealer.setsockopt(zmq.HEARTBEAT_TIMEOUT, 60000)

        # Connect to server
        try:
            self.dealer.connect(self.config.server_addr)
            print(f"[CLIENT] Connected to {self.config.server_addr}", flush=True, file=sys.stderr)
        except Exception as e:
            print(f"[CLIENT] Error connecting to server: {e}", flush=True, file=sys.stderr)
            raise

        # Send HELLO message to initiate handshake
        client_id = self.config.client_id or f"client_{int(time.time() * 1000)}"

        hello_frames = [
            PROTOCOL_VERSION,           # Frame 0: Protocol version byte (b'\x01')
            bytes([MSG_TYPES["HELLO"]]),  # Frame 1: Msg type as single byte (0x01)
            zmq_msgpack.packb({
                "client_id": client_id,
                "resume_session": self.config.resume_session,
            }),
        ]

        try:
            # Send HELLO message - this should work with plain TCP in Python 3.14
            self.dealer.send_multipart(hello_frames)
            print(f"[CLIENT] HELLO sent successfully", flush=True, file=sys.stderr)
        except Exception as e:
            print(f"[CLIENT] Error sending HELLO message: {type(e).__name__}: {e}", flush=True, file=sys.stderr)
            raise

        # Block on recv_multipart - DEALER will receive the ROUTER response
        try:
            frames = self.dealer.recv_multipart()
            print(f"[CLIENT] Received response ({len(frames)} total frames)", flush=True, file=sys.stderr)

            # Check for identity frame (ROUTER adds sender address as first empty frame)
            if len(frames) > 0 and len(frames[0]) == 0:
                # Remove ROUTER prepended identity
                frames = frames[1:]

            print(f"[CLIENT] Received HELLO_ACK ({len(frames)} protocol frames)", flush=True, file=sys.stderr)
        except zmq.Again:
            print("[CLIENT] recv_multipart timed out", flush=True, file=sys.stderr)
            raise
        except Exception as e:
            print(f"[CLIENT] Error in HELLO handshake: {type(e).__name__}: {e}", flush=True, file=sys.stderr)
            traceback.print_exc()
            raise

        # Protocol format for ROUTER response: [version_byte, msg_type_byte, headers[, payload]]
        if len(frames) < 3:
            print(f"[CLIENT] Insufficient frames for HELLO_ACK response", flush=True, file=sys.stderr)
            raise RuntimeError("Invalid HELLO_ACK response from server")

        version = frames[0]
        msg_type_raw = frames[1]
        headers_raw = frames[2]

        msg_type_int = zmq_msgpack.unpackb(msg_type_raw, raw=True)
        print(f"[CLIENT] Received HELLO_ACK: msg_type={msg_type_int}", flush=True, file=sys.stderr)

        if msg_type_int != 2:
            error_msg = str(zmq_msgpack.unpackb(headers_raw)).replace('\n', ' ')[:100]
            print(f"[CLIENT] Expected HELLO_ACK but got msg_type {msg_type_int}: {error_msg}", flush=True, file=sys.stderr)
            raise RuntimeError(f"Expected HELLO_ACK (type=2), got type={msg_type_int}")

        try:
            headers = zmq_msgpack.unpackb(headers_raw)
            self.session_id = headers.get("session_id", "")
            print(f"[CLIENT] Hello acknowledged. Session ID: {self.session_id}", flush=True, file=sys.stderr)

            # Verify session_id is populated for resume_session mode
            if self.config.resume_session and not self.session_id:
                raise RuntimeError("Client session_id is empty after HELLO_ACK (resume_session=True)")
        except Exception as e:
            print(f"[CLIENT] Error unpacking HELLO_ACK headers: {type(e).__name__}: {e}", flush=True, file=sys.stderr)
            raise
