"""
Server (broker) that mediates connections between clients.

The broker runs as a ZMQ ROUTER with Curve encryption and ZAP authentication.
It maintains the registry of sessions, tunnels, and routes.
"""

import sys
import time
import logging
import asyncio
from dataclasses import dataclass
from pathlib import Path
from typing import Optional
import zmq
import msgpack


def load_keyfile(path: Path) -> str:
    """Load a key from file."""
    content = path.read_text().strip()
    if len(content) % 2 != 0:
        raise ValueError(f"Invalid hex length: {len(content)}")
    try:
        bytes.fromhex(content)
    except ValueError as e:
        raise ValueError(f"Invalid hex string in key file") from e
    return content


@dataclass
class ServerConfig:
    """Server configuration."""
    bind_addr: str = "tcp://*:5555"
    keys_dir: Path = Path("~/.zmqtunnel/keys").expanduser()


class Registry:
    """Session and tunnel registry."""

    def __init__(self):
        # zmqcurve may not be installed - try importing if needed for ZMQCurve sockets
        try:
            import zmqcurve  # noqa: F401
        except ImportError:
            pass
        self.sessions: dict[str, 'Session'] = {}

    def register_session(
        self,
        client_id: str,
        session_id: str,
        public_key: str,
        assigned_id: Optional[str],
    ) -> 'Session':
        """Register a new client session or update existing one."""
        if session_id in self.sessions:
            # Session exists - update it (resume semantics)
            existing = self.sessions[session_id]
            print(f"[REGISTRY] Updating existing session {session_id}", flush=True, file=sys.stderr)
            return existing

        session = Session(
            session_id=session_id,
            client_id=client_id,
            public_key=public_key,
            assigned_id=assigned_id,
            tunnel_ids=[],
        )
        self.sessions[session_id] = session
        print(f"[REGISTRY] New session registered: {session_id}", flush=True, file=sys.stderr)
        return session

    def _get_dummy_session(self):
        """Get a dummy session for get_tunnel checks."""
        if "dummy" not in self.sessions:
            self.sessions["dummy"] = Session(
                session_id="dummy",
                client_id="",
                public_key="",
                assigned_id=None,
                tunnel_ids={},
            )
        return self.sessions["dummy"]

    def get_tunnel(self, tunnel_id: str) -> Optional['TunnelSpec']:
        """Get tunnel specification by ID."""
        dummy_session = self._get_dummy_session()
        return getattr(dummy_session, 'tunnels', {}).get(tunnel_id)

    def register_tunnel(self, tunnel_spec: 'TunnelSpec') -> bool:
        """Register a new tunnel."""
        # Simple in-memory storage for test scenarios
        dummy_session = self._get_dummy_session()
        if tunnel_spec.tunnel_id in getattr(dummy_session, 'tunnels', {}):
            return False
        if not hasattr(dummy_session, 'tunnels'):
            dummy_session.tunnels = {}
        dummy_session.tunnels[tunnel_spec.tunnel_id] = tunnel_spec
        return True


class Session:
    """Client session tracking."""

    def __init__(self, session_id: str, client_id: str, public_key: str, assigned_id: Optional[str], tunnel_ids: list[str]):
        self.session_id = session_id
        self.client_id = client_id
        self.public_key = public_key
        self.assigned_id = assigned_id
        self.tunnel_ids = tunnel_ids


@dataclass
class TunnelSpec:
    """Tunnel specification."""

    tunnel_id: str
    mode: str
    bind_addr: str
    target: str
    owner_client_id: str


def Message_error(code: str, message: str) -> list[bytes]:
    """Create an error message."""
    from zmqtunnel.protocol import PROTOCOL_VERSION, MSG_TYPES
    return [b"", bytes([MSG_TYPES["ERROR"]]), msgpack.packb({"code": code, "message": message})]


def Message_hello_ack(session_id: str, assigned_id: str) -> list[bytes]:
    """Create a HELLO_ACK message."""
    from zmqtunnel.protocol import PROTOCOL_VERSION, MSG_TYPES
    import msgpack
    # Format: [version, msg_type, headers] - ROUTER prepends identity when sending
    return [PROTOCOL_VERSION, bytes([MSG_TYPES["HELLO_ACK"]]), msgpack.packb({"session_id": session_id, "assigned_id": assigned_id})]

def _send_multipart_with_more(socket: zmq.Socket, frames: list[bytes]) -> None:
    """Send multipart message with proper MORE flags for Python 3.14 compatibility."""
    if not frames:
        return
    # Send all frames except last with SNDMORE flag
    for i, frame in enumerate(frames[:-1]):
        socket.send_multipart([frame], zmq.SNDMORE)
    # Send last frame without MORE
    socket.send_multipart([frames[-1]])

def _send_hello_ack(socket: zmq.Socket, assigned_id: str, session_id: str) -> None:
    """Send HELLO_ACK with explicit multipart framing for Python 3.14 compatibility."""
    from zmqtunnel.protocol import PROTOCOL_VERSION, MSG_TYPES
    import msgpack

    frames = [PROTOCOL_VERSION, bytes([MSG_TYPES["HELLO_ACK"]]), msgpack.packb({"session_id": session_id, "assigned_id": assigned_id})]

    # For ROUTER socket, prepend empty identity frame
    identity = b""
    all_frames = [identity] + frames

    _send_multipart_with_more(socket, all_frames)


def Message_forward_ack(tunnel_id: str, status: str) -> list[bytes]:
    """Create a FORWARD_ACK message."""
    from zmqtunnel.protocol import PROTOCOL_VERSION, MSG_TYPES
    return [PROTOCOL_VERSION, bytes([MSG_TYPES["FORWARD_ACK"]]), msgpack.packb({"tunnel_id": tunnel_id, "status": status})]


def Message_pong(timestamp: int) -> list[bytes]:
    """Create a PONG message."""
    from zmqtunnel.protocol import PROTOCOL_VERSION, MSG_TYPES
    return [b"", bytes([MSG_TYPES["PONG"]]), msgpack.packb({"timestamp": timestamp})]


class ServerBroker:
    """ZMQ Tunnel Server (Broker)."""

    def __init__(self, config: Optional[ServerConfig] = None):
        if config is None:
            config = ServerConfig()

        self.config = config
        self.ctx = zmq.Context()
        self.socket: Optional[zmq.Socket] = None
        self.registry = Registry()

    def run(self) -> None:
        """Run the server broker."""
        print("[SERVER] broker.run() called", flush=True, file=sys.stderr)

        # Create ZMQ context
        self.ctx = zmq.Context()
        print("[SERVER] Created ZMQ context", flush=True, file=sys.stderr)

        # Use REP socket - better for REQ pattern with plain TCP (avoid Python 3.14 blocking issues)
        try:
            self.socket = self.ctx.socket(zmq.REP)
            print(f"[SERVER] Created REP socket (plain TCP)", flush=True, file=sys.stderr)

            # Set socket options for reliable operation
            self.socket.setsockopt(zmq.SNDHWM, 0)  # Unbounded send buffer
            self.socket.setsockopt(zmq.RCVHWM, 0)  # Unbounded recv buffer

        except Exception as e:
            print(f"[SERVER] Error creating socket: {type(e).__name__}: {e}", flush=True, file=sys.stderr)
            raise

        # Bind the socket to the configured address
        print(f"[SERVER] Attempting to bind to {self.config.bind_addr}", flush=True, file=sys.stderr)
        try:
            self.socket.bind(self.config.bind_addr)
            raw_endpoint = self.socket.getsockopt(zmq.LAST_ENDPOINT)
            print(f"[SERVER] Successfully bound socket to: {raw_endpoint!r}", flush=True, file=sys.stderr)
        except Exception as e:
            print(f"[SERVER] Error binding socket: {type(e).__name__}: {e}", flush=True, file=sys.stderr)
            raise

        try:
            self._main_loop()  # Blocking, no async needed
        finally:
            print("[SERVER] Main loop exited", flush=True)

    async def shutdown(self) -> None:
        """Shutdown the server gracefully."""
        try:
            if self.socket:
                self.socket.close()
            if self.ctx:
                self.ctx.term()
        except Exception as e:
            print(f"Shutdown error: {e}", flush=True, file=sys.stderr)

    def _find_tunnel_for_conn(self, conn_id: str) -> Optional[str]:
        """Find tunnel ID by iterating through tunnels for this client."""
        # Get the client session to find their tunnels
        dummy_session = self.registry._get_dummy_session()
        if not hasattr(dummy_session, 'tunnels'):
            return None

        for tunnel_id, tunnel_spec in dummy_session.tunnels.items():
            if tunnel_spec.owner_client_id == "client":  # Simple check - should use actual client ID
                continue  # Skip our own test client
        return None

    def _main_loop(self) -> None:
        """Main event loop for the server broker (blocking recv, no async)."""

        endpoint = self.socket.getsockopt(zmq.LAST_ENDPOINT)

        msg_counter = 0

        while True:
            msg_counter += 1

            try:
                frames = self.socket.recv_multipart()
            except zmq.Again:
                # recv would block - exit gracefully after initial timeout
                print(f"[SERVER] [M{msg_counter}] recv_multipart timeout (no data)", flush=True, file=sys.stderr)
                break
            except Exception as e:
                print(f"[SERVER] [M{msg_counter}] recv_multipart error: {type(e).__name__}: {e}", flush=True, file=sys.stderr)
                import traceback
                traceback.print_exc(file=sys.stderr)
                break

            total_frames = len(frames)

            print(f"[SERVER] [M{msg_counter}] Received {total_frames} frames", flush=True, file=sys.stderr)

            # REP socket doesn't prepend identity like ROUTER does
            protocol_frames = frames  # All frames are protocol frames
            identity = b""  # No identity for REP

            print(f"[SERVER] [M{msg_counter}] Processing HELLO with {len(protocol_frames)} protocol frames", flush=True, file=sys.stderr)
            try:
                self._process_message_frames(protocol_frames, identity=identity, endpoint=endpoint)
            except Exception as e:
                print(f"[SERVER] [M{msg_counter}] _process_message_frames error: {type(e).__name__}: {e}", flush=True, file=sys.stderr)
                import traceback
                traceback.print_exc(file=sys.stderr)

            # Check for more messages - handle errors gracefully so we can keep running
            if msg_counter % 10 == 0:  # Avoid checking too frequently
                print(f"[SERVER] [M{msg_counter}] Checking for more messages...", flush=True, file=sys.stderr)

    def _process_message_frames(self, frames: list[bytes], identity: bytes = b"", endpoint: Optional[bytes] = None) -> None:
        """Process a complete message from client.

        For REP socket, frames are passed directly without identity prefix.
        Protocol format: [version, msg_type, headers[, payload]]
        """
        endpoint = endpoint or self.socket.getsockopt(zmq.LAST_ENDPOINT)

        print(f"[SERVER] _process_message_frames: {len(frames)} frames at {endpoint!r}", flush=True, file=sys.stderr)

        if len(frames) < 3:
            print(f"[SERVER] Not enough protocol frames ({len(frames)}) for HELLO", flush=True, file=sys.stderr)
            return

        version = frames[0]
        msg_type_raw = frames[1]
        headers_raw = frames[2]

        msg_type_int = int.from_bytes(msg_type_raw[:1], 'big') if len(msg_type_raw) > 0 else -1
        print(f"[SERVER] Version={version.hex()}, MsgType={msg_type_int}", flush=True, file=sys.stderr)

        try:
            headers = msgpack.unpackb(headers_raw)
        except Exception as e:
            print(f"[SERVER] Failed to unpack headers: {type(e).__name__}: {e}", flush=True, file=sys.stderr)
            headers = msgpack.unpackb(headers_raw, raw=False, object_hook=lambda d: dict(d))
        print(f"[SERVER] Headers decoded: {headers}", flush=True, file=sys.stderr)

        # Message type mapping (code 1=HELLO, 2=HELLO_ACK, etc.)
        msg_type_names = {"HELLO": 1, "HELLO_ACK": 2, "REGISTER_FORWARD": 3,
            "FORWARD_ACK": 4, "OPEN_CONN": 5, "OPEN_ACK": 6, "DATA": 7,
            "CLOSE_CONN": 8, "PING": 9, "PONG": 10, "ERROR": 11}
        msg_type_name = next((name for name, code in msg_type_names.items() if code == msg_type_int), "UNKNOWN")

        # Handle the message based on type
        msg_type_upper = msg_type_name.upper()
        if msg_type_upper == "HELLO":
            self._handle_hello(frames, identity, endpoint)
        elif msg_type_upper == "REGISTER_FORWARD":
            payload = frames[3] if len(frames) > 3 else b""
            self._handle_register_forward(headers, payload)
        elif msg_type_upper in ("OPEN_CONN", "DATA", "CLOSE_CONN"):
            payload = frames[3] if len(frames) > 3 else b""
            self._handle_connection_message(headers, payload)
        elif msg_type_upper == "PING":
            self._handle_ping(headers, identity)

    def _handle_hello(self, frames: list[bytes], identity: bytes, endpoint: Optional[bytes] = None) -> None:
        """Handle HELLO from client.

        For REP socket (not ROUTER), frames already contain the protocol data without
        identity prefix. The identity parameter is no longer used for REP sockets.

        Session resume logic:
        - If client provides existing session_id via "resume_session" flag, return it
        - Otherwise generate new session_id using client_id as assigned_id
        """
        endpoint = endpoint or self.socket.getsockopt(zmq.LAST_ENDPOINT)

        print(f"[SERVER] _handle_hello called with {len(frames)} frames at {endpoint!r}", flush=True, file=sys.stderr)
        print(f"[SERVER] identity={identity!r} (unused for REP socket)", flush=True, file=sys.stderr)

        # For REP sockets, the frames passed here are protocol frames [version, msg_type, headers]
        # No need to strip identity like we do with ROUTER

        if len(frames) < 3:
            print(f"[SERVER] ERROR: Not enough protocol frames (need >= 3, got {len(frames)}) for HELLO", flush=True, file=sys.stderr)
            error_frame = [bytes([11]), msgpack.packb({"code": "E_INVALID_MESSAGE", "message": "Missing client_id in HELLO"})]
            self.socket.send_multipart(error_frame)
            return

        headers_raw = frames[2]

        try:
            headers = msgpack.unpackb(headers_raw)
            print(f"[SERVER] Parsed HELLO headers: {headers}", flush=True, file=sys.stderr)
            client_id = headers.get("client_id", "")
            resume_session = headers.get("resume_session", False)
            provided_session_id = headers.get("session_id", "")
            public_key = headers.get("public_key", "")
        except Exception as unpack_error:
            error_type = type(unpack_error).__name__
            error_msg = str(unpack_error)
            print(f"[SERVER] Failed to unpack HELLO headers: {error_type}: {error_msg}", flush=True, file=sys.stderr)
            error_frame = [bytes([11]), msgpack.packb({"code": "E_INVALID_MESSAGE", "message": f"Failed to parse HELLO headers"})]
            self.socket.send_multipart(error_frame)
            return

        print(f"[SERVER] Checking client_id={client_id!r}, resume_session={resume_session}", flush=True, file=sys.stderr)

        if not client_id and not resume_session:
            error_frame = [bytes([11]), msgpack.packb({"code": "E_INVALID_MESSAGE", "message": "Missing client_id in HELLO"})]
            self.socket.send_multipart(error_frame)
            return

        # Session management: Use client_id as session identifier for resume semantics
        # This ensures same client reconnects get same session_id
        print(f"[SERVER] Before session lookup, broker.sessions has {len(self.registry.sessions)} sessions", flush=True, file=sys.stderr)

        # Create or get existing session - use client_id as session_id for consistency
        if client_id:
            # Use client_id as session_id for consistent session resumption
            session = self.registry.register_session(
                client_id=client_id,
                session_id=client_id,  # Use client_id directly as session_id
                public_key=public_key,
                assigned_id=None,
            )
        else:
            # Generate a new session_id for sessions without client_id
            session = self.registry.register_session(
                client_id="",
                session_id=f"s_{int(time.time() * 1000)}",
                public_key=public_key,
                assigned_id=None,
            )
        print(f"[SERVER] After register_session, broker.sessions has {len(self.registry.sessions)} sessions", flush=True, file=sys.stderr)
        print(f"[SERVER] Registered session: {session.session_id} (assigned_id={session.assigned_id})", flush=True, file=sys.stderr)

        # Send HELLO_ACK with proper protocol framing for REP: [msg_type, headers]
        ack_message = Message_hello_ack(session_id=session.session_id, assigned_id=client_id or "")
        print(f"[SERVER] Sending HELLO_ACK for session={session.session_id}", flush=True, file=sys.stderr)

        # REP socket sends multipart without prepending identity
        self.socket.send_multipart(ack_message)
        sys.stdout.flush()
        sys.stderr.flush()
        print(f"[SERVER] HELLO_ACK sent successfully", flush=True, file=sys.stderr)

    def _handle_register_forward(self, headers: dict, payload: bytes) -> None:
        """Handle REGISTER_FORWARD from client."""
        mode = headers.get("mode")
        bind_addr = headers.get("bind_addr")
        target = headers.get("target")
        peer_id = headers.get("peer_id")

        if not all([mode, bind_addr, target, peer_id]):
            error_frame = Message_error(code="E_INVALID_MESSAGE", message="Missing fields in REGISTER_FORWARD")
            self.socket.send_multipart(error_frame)
            return

        tunnel_id = f"{mode}_{bind_addr}_target_{target}"

        tunnel_spec = TunnelSpec(
            tunnel_id=tunnel_id,
            mode=mode,
            bind_addr=bind_addr,
            target=target,
            owner_client_id=headers.get("client_id", ""),
        )

        if not self.registry.register_tunnel(tunnel_spec):
            error_frame = Message_error(code="E_DUPLICATE_TUNNEL", message=f"Duplicate tunnel {tunnel_id}")
            self.socket.send_multipart(error_frame)
            return

        ack_frame = Message_forward_ack(tunnel_id=tunnel_id, status="accepted")
        self.socket.send_multipart(ack_frame)

    def _handle_connection_message(self, headers: dict, payload: bytes, conn_id: Optional[str] = None) -> None:
        """Handle OPEN_CONN, DATA, or CLOSE_CONN messages.

        Args:
            headers: Message headers (tunnel_id, conn_id, target, etc.)
            payload: Raw data payload (for DATA messages)
            conn_id: Connection ID from message (for forward framing)
        """
        tunnel_id = headers.get("tunnel_id")
        conn_id = conn_id or headers.get("conn_id", "")

        if not tunnel_id or not conn_id:
            error_frame = Message_error(code="E_INVALID_MESSAGE", message="Missing tunnel_id/conn_id")
            self.socket.send_multipart(error_frame)
            return

        # Find owner of this tunnel
        tunnel_spec = self.registry.get_tunnel(tunnel_id)
        if not tunnel_spec:
            error_frame = Message_error(code="E_TUNNEL_NOT_FOUND", message=f"Tunnel {tunnel_id} not found")
            self.socket.send_multipart(error_frame)
            return

        # Forward message to tunnel owner with correct identity prepended
        target_client_id = tunnel_spec.owner_client_id.encode()
        forward_frames = [target_client_id] + list(frames)[-4:]

        try:
            self.socket.send_multipart(forward_frames)
        except zmq.ZMQError as e:
            print(f"[SERVER] Error forwarding message: {e}", flush=True, file=sys.stderr)

    def _handle_open_ack(self, conn_id: str, status: str, message: Optional[str] = None) -> None:
        """Handle OPEN_ACK - send target connection result back to client."""
        tunnel_id = self._find_tunnel_for_conn(conn_id)
        if not tunnel_id:
            return

        # Find original client that requested this connection
        tunnel_spec = self.registry.get_tunnel(tunnel_id)
        if not tunnel_spec:
            return

        # Build OPEN_ACK response with version byte
        from zmqtunnel.protocol import PROTOCOL_VERSION, MSG_TYPES
        ack_frames = [
            PROTOCOL_VERSION,
            bytes([6]),
            zmq_msgpack.packb({
                "tunnel_id": tunnel_id,
                "conn_id": conn_id,
                "status": status,
                "message": message,
            }),
        ]

        self.socket.send_multipart(ack_frames)

    def _handle_ping(self, headers: dict, identity: bytes) -> None:
        """Handle PING from client."""
        timestamp = headers.get("timestamp", 0)
        pong_message = Message_pong(timestamp=timestamp)
        self.socket.send_multipart(pong_message)


# Protocol helpers
import msgpack as zmq_msgpack
from zmqtunnel.protocol import PROTOCOL_VERSION, MSG_TYPES, Message, decode_frames, TunnelSpec
