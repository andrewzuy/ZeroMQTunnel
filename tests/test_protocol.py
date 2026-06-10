"""Tests for the protocol module."""

import pytest
from zmqtunnel.protocol import (
    PROTOCOL_VERSION,
    MSG_TYPES,
    Message,
    decode_frames,
    TunnelSpec,
)


class TestMessageCreation:
    """Test message creation methods."""

    def test_hello(self):
        frames = Message.hello("client123", auth_token="token456", resume=False)
        assert len(frames) == 3
        _, msg_type, headers = decode_frames(frames)[:3]
        assert headers["client_id"] == "client123"
        assert headers["resume_session"] is False

    def test_hello_with_auth(self):
        frames = Message.hello("client123", auth_token="token456")
        _, msg_type, headers = decode_frames(frames)[:3]
        assert headers["auth_token"] == "token456"

    def test_hello_ack(self):
        frames = Message.hello_ack("sess_abc123", assigned_id="client456")
        result = decode_frames(frames)
        assert "session_id" in result[2]

    def test_register_forward(self):
        frames = Message.register_forward("L", "tcp://*:8080", "http://target:80", "peer_xyz")
        result = decode_frames(frames)
        headers = result[2]
        assert headers["mode"] == "L"
        assert headers["bind_addr"] == "tcp://*:8080"

    def test_forward_ack(self):
        frames = Message.forward_ack("tunnel_1", status="accepted")
        _, msg_type, headers = decode_frames(frames)[:3]
        assert headers["status"] == "accepted"

    def test_open_conn(self):
        frames = Message.open_conn("tunnel_1", "conn_aaa", "http://localhost:8080")
        result = decode_frames(frames)
        headers = result[2]
        assert headers["tunnel_id"] == "tunnel_1"
        assert headers["conn_id"] == "conn_aaa"

    def test_open_ack(self):
        frames = Message.open_ack("conn_bbb", status="success")
        _, msg_type, headers = decode_frames(frames)[:3]
        assert headers["status"] == "success"

    def test_close_conn(self):
        frames = Message.close_conn("conn_ccc", reason="client_closed")
        result = decode_frames(frames)
        headers = result[2]
        assert headers["reason"] == "client_closed"

    def test_ping(self):
        frames = Message.ping(timestamp=1234567890)
        _, msg_type, headers = decode_frames(frames)[:3]
        assert headers["timestamp"] == 1234567890

    def test_pong(self):
        frames = Message.pong(timestamp=9876543210)
        result = decode_frames(frames)
        assert result[3] is None  # No payload for PONG

    def test_error(self):
        frames = Message.error(code="E_AUTH_FAILED", message="Invalid client key")
        _, msg_type, headers = decode_frames(frames)[:3]
        assert headers["code"] == "E_AUTH_FAILED"


class TestConstants:
    """Test protocol constants."""

    def test_protocol_version(self):
        assert PROTOCOL_VERSION == b"\x01"

    def test_msg_types_defined(self):
        assert "HELLO" in MSG_TYPES
        assert "HELLO_ACK" in MSG_TYPES
        assert "OPEN_CONN" in MSG_TYPES
        assert "DATA" in MSG_TYPES
        assert "CLOSE_CONN" in MSG_TYPES


class TestTunnelSpec:
    """Test TunnelSpec class."""

    def test_tunnel_spec_creation(self):
        spec = TunnelSpec(
            tunnel_id="test_tunnel",
            mode="L",
            bind_addr="tcp://*:8080",
            target="http://target:80",
            owner_client_id="client123"
        )
        assert spec.tunnel_id == "test_tunnel"
        assert spec.mode == "L"
        assert spec.bind_addr == "tcp://*:8080"
        assert spec.target == "http://target:80"


class TestDecodeFrames:
    """Test frame decoding."""

    def test_extra_payload_frame(self):
        """DATA messages have an optional payload frame"""
        frames = Message.data("conn_test", seq=42, payload=b"hello")
        result = decode_frames(frames)
        assert len(result) == 4  # version, type, headers, payload
        assert result[3] == b"hello"
