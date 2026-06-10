"""
Client agent for local and remote forwarding modes.

Simplified implementation focusing on the core forwarding logic.
"""

import asyncio
import errno
import socket
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional, Dict, Union
import zmq
import msgpack as zmq_msgpack
import traceback

# Python 3.14 compatibility - get_event_loop deprecated
def get_event_loop():
    loop = asyncio.get_running_loop()
    return loop


@dataclass
class ClientConfig:
    """Client configuration."""
    server_addr: str = "tcp://localhost:5555"
    server_key_path: Path = Path("~/.zmqtunnel/server_public.key").expanduser()
    client_key_path: Path = Path("~/.zmqtunnel/client_secret.key").expanduser()

    # Forwarding modes - only one can be set at a time
    local_bind: Optional[str] = None   # For -L mode (e.g., "tcp://*:8080")
    remote_bind: Optional[str] = None  # For -R mode (e.g., "tcp://*:9090")

    resume_session: bool = False
    client_id: str = ""

    def __init__(self, *, server_addr: Optional[str] = None,
                 server_key: Optional[Union[Path, str]] = None,
                 server_key_path: Optional[Path] = None,
                 client_key: Optional[Union[Path, str]] = None,
                 client_key_path: Optional[Path] = None,
                 local_bind: Optional[str] = None,
                 remote_bind: Optional[str] = None,
                 resume_session: bool = False,
                 client_id: str = "") -> None:
        """Initialize ClientConfig with flexible argument handling."""

        # Handle both server_key and server_key_path
        if server_key is not None:
            self.server_key_path = Path(server_key) if isinstance(server_key, str) else server_key
        elif server_key_path is not None:
            self.server_key_path = server_key_path

        # Handle both client_key and client_key_path
        if client_key is not None:
            self.client_key_path = Path(client_key) if isinstance(client_key, str) else client_key
        elif client_key_path is not None:
            self.client_key_path = client_key_path

        if server_addr is not None:
            self.server_addr = server_addr
        if local_bind is not None:
            self.local_bind = local_bind
        if remote_bind is not None:
            self.remote_bind = remote_bind
        if resume_session is not False:  # Allow explicit False to override default
            self.resume_session = resume_session
        if client_id != "":
            self.client_id = client_id


class ClientAgent:
    """
    ZMQ Tunnel Client.

    Handles connections to the server and manages TCP forwarding.
    Supports both local forwarding (-L) and remote forwarding (-R) modes.
    """

    def __init__(self, config: Optional[ClientConfig] = None):
        if config is None:
            config = ClientConfig()

        self.config = config
        self.ctx = zmq.Context()
        self.dealer: Optional[zmq.Socket] = None

        # ZMQ_STREAM sockets for TCP connections (multiplexed over the dealer link)
        self.local_stream: Optional[zmq.Socket] = None  # Listener for local (-L) mode
        self.remote_stream: Optional[zmq.Socket] = None  # Listener for remote (-R) mode

        # Connection tracking: conn_id -> {target_addr, stream_identity}
        self.connections: Dict[str, dict] = {}

        # Session info from HELLO_ACK
        self.session_id: str = ""

    def connect_and_authenticate(self) -> None:
        """Initialize ZMQ connection to server and complete handshake.

        Python 3.14 workaround: Use REQ socket instead of DEALER for proper recv_multipart() behavior.
        With plain TCP, the original Python 3.14 bug causes indefinite blocking on recv_multipart().

        The REQ socket type has better state tracking even with Python 3.14's broken libzmq backend.
        """
        from zmqtunnel.protocol import PROTOCOL_VERSION, MSG_TYPES

        # Use REQ socket - Python 3.14 bug affects DEALER more than REQ due to socket type differences.
        self.dealer = self.ctx.socket(zmq.REQ)
        print("[CLIENT] Using REQ socket", flush=True, file=sys.stderr)

        # Set socket options for reliable operation
        self.dealer.setsockopt(zmq.SNDHWM, 0)  # Unbounded send buffer
        self.dealer.setsockopt(zmq.RCVHWM, 0)  # Unbounded recv buffer
        try:
            self.dealer.setsockopt(zmq.TCP_NODELAY, True)
        except Exception:
            pass

        # Connect to server
        try:
            self.dealer.connect(self.config.server_addr)
            print(f"[CLIENT] Connected to {self.config.server_addr}", flush=True, file=sys.stderr)
        except Exception as e:
            print(f"[CLIENT] Error connecting to server: {e}", flush=True, file=sys.stderr)
            raise

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
            # Send HELLO message via REQ socket
            self.dealer.send_multipart(hello_frames)
            print(f"[CLIENT] HELLO sent successfully", flush=True, file=sys.stderr)
        except Exception as e:
            print(f"[CLIENT] Error sending HELLO message: {type(e).__name__}: {e}", flush=True, file=sys.stderr)
            raise

        try:
            # REQ socket recv_multipart should work even with Python 3.14 due to different C-state handling
            received_frames = self.dealer.recv_multipart()

            print(f"[CLIENT] Received response ({len(received_frames)} total frames)", flush=True, file=sys.stderr)

            # Check for identity frame (ROUTER adds sender address as first empty frame)
            if len(received_frames) > 0 and len(received_frames[0]) == 0:
                # Remove ROUTER prepended identity
                frames = received_frames[1:]
            else:
                frames = received_frames

            print(f"[CLIENT] Received HELLO_ACK ({len(frames)} protocol frames)", flush=True, file=sys.stderr)
        except TimeoutError:
            print("[CLIENT] recv_multipart timed out - server did not respond", flush=True, file=sys.stderr)
            raise TimeoutError("No response from server")
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

    async def _handle_server_message(self, frames: list[bytes]) -> None:
        """Handle a message from the server."""
        if len(frames) < 3:
            return

        # Strip empty identity frame (ROUTER prepends sender address)
        if len(frames) > 0 and len(frames[0]) == 0:
            frames = frames[1:]

        version = frames[0]
        msg_type_raw = frames[1]
        headers_raw = frames[2]
        payload = frames[3] if len(frames) > 3 else b""

        msg_type_int = zmq_msgpack.unpackb(msg_type_raw, raw=True)
        headers_dict = zmq_msgpack.unpackb(headers_raw)

        msg_type_names = {
            1: "HELLO", 2: "HELLO_ACK", 3: "REGISTER_FORWARD",
            4: "FORWARD_ACK", 5: "OPEN_CONN", 6: "OPEN_ACK", 7: "DATA",
            8: "CLOSE_CONN", 9: "PING", 10: "PONG", 11: "ERROR"
        }
        msg_type_name = msg_type_names.get(msg_type_int, "UNKNOWN")

        try:
            if msg_type_name == "HELLO_ACK":
                self.session_id = headers_dict.get("session_id", "")
                print(f"[CLIENT] Hello acknowledged. Session ID: {self.session_id}", flush=True)
                sys.stdout.flush()
            elif msg_type_name == "FORWARD_ACK":
                tunnel_id = headers_dict.get("tunnel_id", "")
                status = headers_dict.get("status", "")
                if status == "accepted":
                    print(f"[CLIENT] Forward registered: {tunnel_id}", flush=True)
                else:
                    print(f"[CLIENT] Forward rejected: {tunnel_id}", flush=True)
            elif msg_type_name == "OPEN_CONN":
                tunnel_id = headers_dict.get("tunnel_id", "")
                conn_id = headers_dict.get("conn_id", "")
                target = headers_dict.get("target", "")

                if target:
                    await self._handle_open_conn(tunnel_id, conn_id, target)
            elif msg_type_name == "OPEN_ACK":
                conn_id = headers_dict.get("conn_id", "")
                status = headers_dict.get("status", "")
                message = headers_dict.get("message", "")

                print(f"[CLIENT] OPEN_ACK for {conn_id}: {status}", flush=True)
                if message:
                    print(f"[CLIENT]   Message: {message}", flush=True)
            elif msg_type_name == "DATA":
                conn_id = headers_dict.get("conn_id", "")
                seq = int(headers_dict.get("seq", 0))
                payload_data = frames[-1] if len(frames) > 3 else b""
                await self.handle_data(conn_id, payload_data)
            elif msg_type_name == "CLOSE_CONN":
                conn_id = headers_dict.get("conn_id", "")
                reason = headers_dict.get("reason", "closed")
                print(f"[CLIENT] Connection {conn_id} closed: {reason}", flush=True)

        except Exception as e:
            print(f"Error handling server message: {e}", flush=True, file=sys.stderr)
            traceback.print_exc()

    async def _listener_loop(self) -> None:
        listener = self.local_stream or self.remote_stream
        if not listener:
            return

        poller = zmq.Poller()
        poller.register(listener, zmq.POLLIN)

        try:
            while True:
                socks = dict(poller.poll(timeout=100))

                if listener in socks and socks[listener] == zmq.POLLIN:
                    try:
                        # Receive connection notification from ZMQ_STREAM
                        evt_type, _, _, last_endpoint = listener.recv_multipart()

                        if evt_type != b"EVENT_CONNECT":
                            continue

                        conn_info = f"{evt_type}:{last_endpoint}"
                        print(f"[CLIENT] New connection: {conn_info}", flush=True, file=sys.stderr)

                        # Create connection ID and tunnel ID
                        conn_id = f"c_{int(time.time() * 1000)}"
                        tunnel_id = self._get_tunnel_id(listener)

                        # Send OPEN_CONN to server to start target dial
                        open_frames = [
                            b"",  # Empty identity for DEALER sender
                            bytes([5]),  # OpenConn msg type (0x05)
                            zmq_msgpack.packb({
                                "tunnel_id": tunnel_id,
                                "conn_id": conn_id,
                                "target": f"tcp://localhost:{self._extract_port(listener)}",
                            }),
                        ]

                        await self.dealer.send_multipart(open_frames)
                        print(f"[CLIENT] Sent OPEN_CONN for {conn_id} -> target:tcp://localhost:{self._extract_port(listener)}", flush=True, file=sys.stderr)

                    except Exception as e:
                        print(f"[CLIENT] Listener error handling connect: {e}", flush=True, file=sys.stderr)

        except (zmq.Again, KeyboardInterrupt):
            pass

    def _get_tunnel_id(self, listener: zmq.Socket) -> str:
        """Get tunnel ID based on listener socket."""
        endpoint = listener.getsockopt(zmq.LAST_ENDPOINT, "").strip()
        if not endpoint:
            # Fallback to IDENTITY option if LAST_ENDPOINT is empty
            identity_bytes = listener.getsockopt(zmq.IDENTITY, b"").decode()
            endpoint = f"id:{identity_bytes}"
        return f"T_{endpoint.replace(':', '_')}"

    async def _handle_open_conn(self, tunnel_id: str, conn_id: str, target: str) -> None:
        """Handle OPEN_CONN from server - establish connection to target."""
        import zmq

        # Create DEALER socket for this connection
        stream_sock = self.ctx.socket(zmq.DEALER)
        stream_sock.setsockopt(zmq.IDENTITY, f"stream_{conn_id}".encode())

        target_addr = target

        try:
            import socket
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.connect((target_addr.split(":")[0], int(target_addr.split(":")[-1])))

            # Send DATA message to server via dealer link (connection established)
            data_frames = [
                b"",  # Empty identity
                bytes([7]),  # DATA msg type
                zmq_msgpack.packb({
                    "tunnel_id": tunnel_id,
                    "conn_id": conn_id,
                    "seq": 0,
                }),
                b"",  # Empty payload (connection established)
            ]
            await stream_sock.send_multipart(data_frames)
        except Exception as e:
            print(f"[CLIENT] Error establishing connection: {e}", flush=True, file=sys.stderr)
            raise

    async def handle_data(self, conn_id: str, payload: bytes) -> None:
        """Handle incoming DATA frame from server (forwarded data)."""
        # Deliver raw bytes to tracked connection
        if conn_id in self.connections:
            conn_info = self.connections[conn_id]
            stream_sock = self.ctx.socket(zmq.DEALER)
            stream_sock.setsockopt(zmq.IDENTITY, f"stream_{conn_id}".encode())

            try:
                # Send DATA message back through dealer link for bidirectional forwarding
                data_frames = [
                    b"",  # Empty identity
                    bytes([7]),  # DATA msg type
                    zmq_msgpack.packb({
                        "tunnel_id": conn_info.get("tunnel_id", ""),
                        "conn_id": conn_id,
                        "seq": len(conn_info.get("received_data", [])),
                    }),
                    payload,  # Data payload
                ]
                await stream_sock.send_multipart(data_frames)
            except Exception as e:
                print(f"[CLIENT] Error forwarding data: {e}", flush=True, file=sys.stderr)

    async def _heartbeat_task(self) -> None:
        """Send periodic heartbeats to server."""
        if not self.dealer:
            return

        while True:
            try:
                ping_frames = [
                    b"",  # Empty identity
                    bytes([9]),  # Ping msg type
                    msgpack.packb({"timestamp": int(1e9 * asyncio.get_event_loop().time())}),
                ]
                await self.dealer.send_multipart(ping_frames)

                await asyncio.sleep(30)

            except Exception as e:
                print(f"Heartbeat error: {e}")

    async def run(self) -> None:
        """Main event loop for the client agent.

        Manages:
          - Connection/reconnection to server
          - HELLO handshake
          - Incoming message handling (forward rules, data, etc.)
          - Outgoing connections (OPEN_CONN handling)
          - Heartbeat / keepalive
          - Graceful shutdown
        """
        print("[CLIENT] run() started", flush=True, file=sys.stderr)

        # Ensure we have a session ID (connect_and_authenticate is now synchronous for Python 3.14)
        if not self.session_id and self.config.resume_session:
            self.connect_and_authenticate()

        # Main loop with async gather for concurrent tasks
        try:
            await asyncio.gather(
                self._message_loop(),      # Receive and process messages from server
                self._listener_loop(),     # Handle incoming TCP connections (local/remote mode)
                self._heartbeat_task(),    # Periodic heartbeats
            )
        except Exception as e:
            print(f"[CLIENT] Error in main loop: {e}", flush=True, file=sys.stderr)
            raise

        await self.shutdown()

    async def _message_loop(self) -> None:
        """Receive and process messages from the server."""
        if not self.dealer:
            return

        poller = zmq.Poller()
        poller.register(self.dealer, zmq.POLLIN)

        try:
            while True:
                socks = dict(poller.poll(timeout=100))

                if self.dealer in socks and socks[self.dealer] == zmq.POLLIN:
                    try:
                        frames = await asyncio.wait_for(
                            self.dealer.recv_multipart(),
                            timeout=5.0
                        )

                        # Strip empty identity frame (ROUTER prepends sender address)
                        # REQ socket still uses ROUTER pattern, so this is still needed
                        if len(frames) > 0 and len(frames[0]) == 0:
                            frames = frames[1:]

                        await self._handle_server_message(frames)

                    except asyncio.TimeoutError:
                        # No data available - continue loop
                        continue
                    except zmq.Again:
                        # recv_multipart would block - shouldn't happen with timeout
                        pass
                    except Exception as e:
                        print(f"[CLIENT] Error in message loop: {e}", flush=True, file=sys.stderr)

        except (KeyboardInterrupt, zmq.ZMQError):
            pass

