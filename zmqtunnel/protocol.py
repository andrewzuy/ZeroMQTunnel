"""
Protocol definitions and encoding/decoding for ZMQ tunnel messages.
"""

import zmq
from dataclasses import asdict, dataclass, field
from typing import Any, Optional
import msgpack


# ============================================================================
# Constants
# ============================================================================

PROTOCOL_VERSION = b"\x01"
MSG_TYPES = {
    "HELLO": 0x01,
    "HELLO_ACK": 0x02,
    "REGISTER_FORWARD": 0x03,
    "FORWARD_ACK": 0x04,
    "OPEN_CONN": 0x05,
    "OPEN_ACK": 0x06,
    "DATA": 0x07,
    "CLOSE_CONN": 0x08,
    "PING": 0x09,
    "PONG": 0x0A,
    "ERROR": 0x0B,
}


# ============================================================================
# Message Types
# ============================================================================

@dataclass
class Message:
    """Base message class with type and optional payload."""
    msg_type: int
    headers: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def hello(cls, client_id: str, auth_token: Optional[str] = None, resume: bool = False) -> list[bytes]:
        """Create a HELLO message."""
        headers = {
            "client_id": client_id,
            "resume_session": resume,
        }
        if auth_token:
            headers["auth_token"] = auth_token
        return cls._multipart(PROTOCOL_VERSION, MSG_TYPES["HELLO"], headers)

    @classmethod
    def hello_ack(cls, session_id: str, assigned_id: Optional[str] = None) -> list[bytes]:
        """Create a HELLO_ACK message."""
        headers = {"session_id": session_id}
        if assigned_id:
            headers["assigned_id"] = assigned_id
        return cls._multipart(PROTOCOL_VERSION, MSG_TYPES["HELLO_ACK"], headers)

    @classmethod
    def register_forward(cls, mode: str, bind_addr: str, target: str, peer_id: str) -> list[bytes]:
        """Create a REGISTER_FORWARD message."""
        headers = {
            "mode": mode,  # 'L' or 'R'
            "bind_addr": bind_addr,
            "target": target,
            "peer_id": peer_id,
        }
        return cls._multipart(PROTOCOL_VERSION, MSG_TYPES["REGISTER_FORWARD"], headers)

    @classmethod
    def forward_ack(cls, tunnel_id: str, status: str) -> list[bytes]:
        """Create a FORWARD_ACK message."""
        headers = {"tunnel_id": tunnel_id, "status": status}
        return cls._multipart(PROTOCOL_VERSION, MSG_TYPES["FORWARD_ACK"], headers)

    @classmethod
    def open_conn(cls, tunnel_id: str, conn_id: str, target: str) -> list[bytes]:
        """Create an OPEN_CONN message."""
        headers = {
            "tunnel_id": tunnel_id,
            "conn_id": conn_id,
            "target": target,
        }
        return cls._multipart(PROTOCOL_VERSION, MSG_TYPES["OPEN_CONN"], headers)

    @classmethod
    def open_ack(cls, conn_id: str, status: str) -> list[bytes]:
        """Create an OPEN_ACK message."""
        headers = {"conn_id": conn_id, "status": status}
        return cls._multipart(PROTOCOL_VERSION, MSG_TYPES["OPEN_ACK"], headers)

    @classmethod
    def data(cls, conn_id: str, seq: int, payload: bytes) -> list[bytes]:
        """Create a DATA message."""
        header = {"conn_id": conn_id, "seq": seq}
        return cls._multipart(PROTOCOL_VERSION, MSG_TYPES["DATA"], header, payload)

    @classmethod
    def close_conn(cls, conn_id: str, reason: str = "closed") -> list[bytes]:
        """Create a CLOSE_CONN message."""
        headers = {"conn_id": conn_id, "reason": reason}
        return cls._multipart(PROTOCOL_VERSION, MSG_TYPES["CLOSE_CONN"], headers)

    @classmethod
    def ping(cls, timestamp: int | None = None) -> list[bytes]:
        """Create a PING message."""
        headers = {"timestamp": timestamp or 0}
        return cls._multipart(PROTOCOL_VERSION, MSG_TYPES["PING"], headers)

    @classmethod
    def pong(cls, timestamp: int) -> list[bytes]:
        """Create a PONG message."""
        headers = {"timestamp": timestamp}
        return cls._multipart(PROTOCOL_VERSION, MSG_TYPES["PONG"], headers)

    @classmethod
    def error(cls, code: str, message: str) -> list[bytes]:
        """Create an ERROR message."""
        headers = {"code": code, "message": message}
        return cls._multipart(PROTOCOL_VERSION, MSG_TYPES["ERROR"], headers)

    @staticmethod
    def _multipart(version: bytes, msg_type: int, headers: dict, payload: Optional[bytes] = None) -> list[bytes]:
        """Create a multipart message envelope."""
        result = [version, msgpack.packb(msg_type), msgpack.packb(headers)]
        if payload is not None:
            result.append(payload)
        return result


# ============================================================================
# Encoding / Decoding Functions
# ============================================================================

def create_multipart(version: bytes, msg_type: int, headers: dict, payload: bytes | None = None) -> list[bytes]:
    """Create a multipart message envelope."""
    result = [version, msgpack.packb(msg_type), msgpack.packb(headers)]
    if payload is not None:
        result.append(payload)
    return result


def encode_message(msg: Message) -> list[bytes]:
    """Encode a message to multipart frames."""
    return create_multipart(PROTOCOL_VERSION, msg.msg_type, msg.headers)


def decode_frames(frames: list[bytes]) -> tuple[str, str, dict, bytes | None]:
    """Decode multipart frames into components.

    Args:
        frames: List of frames from ZMQ (should be at least 3: version, type, headers)

    Returns:
        Tuple of (version_hex, msg_type_name, headers_dict, payload).
    """
    if len(frames) < 3:
        raise ValueError(f"Expected at least 3 frames, got {len(frames)}")

    # Frame 0: protocol version (raw bytes)
    version = frames[0]

    # Frame 1: msg type (msgpack-encoded uint8)
    msg_type_int = msgpack.unpackb(frames[1])
    msg_type_name = next((name for name, code in MSG_TYPES.items() if code == msg_type_int), "UNKNOWN")

    # Frame 2: headers (msgpack-encoded dict)
    headers = msgpack.unpackb(frames[2])

    payload = None
    if len(frames) > 3:
        payload = frames[3]

    return version.hex(), msg_type_name, headers, payload


# ============================================================================
# Export classes for other modules to import
# ============================================================================


class TunnelSpec:
    """Specification for a forwarding tunnel."""
    __slots__ = ("tunnel_id", "mode", "bind_addr", "target", "owner_client_id", "status")

    def __init__(
        self,
        tunnel_id: str,
        mode: str,
        bind_addr: str,
        target: str,
        owner_client_id: str,
        status: str = "active",
    ):
        self.tunnel_id = tunnel_id
        self.mode = mode
        self.bind_addr = bind_addr
        self.target = target
        self.owner_client_id = owner_client_id
        self.status = status


# ============================================================================
# Type aliases for convenience
# ============================================================================

MsgType = int
Headers = dict[str, Any]
