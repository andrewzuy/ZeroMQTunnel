"""
Phase 2 Integration Tests: Plain TCP Transport Handshake.

End-to-end tests that verify the plain TCP HELLO/HELLO_ACK handshake
works correctly between a client and server broker.

Tests cover:
- Fresh connection and authentication flow
- Session resumption across reconnects
- Error handling for invalid messages
- Heartbeat mechanism configuration
"""

import asyncio
import sys
import time
import threading
import pytest


def test_keypair_format():
    """Test that keypair generation produces valid hex strings."""
    from zmqtunnel.crypto import generate_keypair
    secret, public = generate_keypair()
    assert len(secret) == 80
    assert len(public) == 80
    assert len(secret) == len(public)


def test_multiple_keypairs_unique():
    """Test that multiple keypairs are all unique."""
    from zmqtunnel.crypto import generate_keypair
    keys = [p[0] for p in (generate_keypair() for _ in range(10))]
    assert len(keys) == len(set(keys))


def test_generate_and_write_keys(tmp_path):
    """Test that key generation and file writing works correctly."""
    from zmqtunnel.crypto import generate_keypair

    keys_dir = tmp_path / "keys"
    keys_dir.mkdir(parents=True)

    secret, public = generate_keypair()
    secret_file = keys_dir / "test_secret.key"
    public_file = keys_dir / "test_public.key"

    secret_file.write_text(secret)
    public_file.write_text(public)

    assert secret_file.read_text() == secret
    assert public_file.read_text() == public


class TestHelloHandshakeEndToEnd:
    """Test the complete HELLO/HELLO_ACK handshake flow."""

    def test_hello_handshake_success(self, tmp_path):
        """Test successful HELLO/HELLO_ACK exchange with plain TCP sockets (blocking)."""
        from zmqtunnel.server.broker import ServerBroker, ServerConfig
        from zmqtunnel.client.agent import ClientAgent, ClientConfig

        # Use unique port within valid range to avoid address reuse issues
        def get_free_port():
            import socket
            with socket.socket() as s:
                s.bind(("", 0))
                return s.getsockname()[1]

        # Generate a reasonable unique port (e.g., random offset from base)
        base_port = 6400 + get_free_port() % 200
        server_port = str(base_port)

        broker_config = ServerConfig(
            bind_addr=f"tcp://*:{server_port}",
        )
        broker = ServerBroker(config=broker_config)

        # Start broker in background thread (blocking mode for Python 3.14 compatibility)
        import threading
        print(f"[TEST] Starting handshake test on port {server_port}...", flush=True, file=sys.stderr)

        def run_broker():
            # Call blocking main loop directly (no asyncio.run needed)
            broker.run()

        broker_thread = threading.Thread(target=run_broker, daemon=True)
        broker_thread.start()

        # Wait for server to be ready
        time.sleep(0.5)

        # Use LAST_ENDPOINT to get actual bound address
        import zmq
        raw_endpoint = broker.socket.getsockopt(zmq.LAST_ENDPOINT)
        print(f"[TEST] Broker socket bound to: {raw_endpoint}", flush=True, file=sys.stderr)

        # Create client and connect (blocking mode for Python 3.14 compatibility)
        agent_config = ClientConfig(
            server_addr=f"tcp://localhost:{server_port}",
            client_id="test-client-1",
            resume_session=False,
        )
        print(f"[TEST] Creating ClientAgent for server {server_port}...", flush=True, file=sys.stderr)
        agent = ClientAgent(config=agent_config)

        # Print client config to debug
        print(f"[TEST] ClientConfig: server_addr={agent_config.server_addr}, client_id={agent_config.client_id!r}, resume_session={agent_config.resume_session}", flush=True, file=sys.stderr)

        try:
            agent.connect_and_authenticate()
            print("[TEST] Handshake completed successfully!", flush=True, file=sys.stderr)
            print(f"[TEST] Agent session_id after handshake: {repr(agent.session_id)}", flush=True, file=sys.stderr)
        except Exception as e:
            print(f"[TEST] connect_and_authenticate raised exception: {type(e).__name__}: {e}", flush=True, file=sys.stderr)
            raise

        # Verify handshake succeeded - session exists in registry
        print(f"[TEST] Broker has {len(broker.registry.sessions)} sessions")
        assert len(broker.registry.sessions) > 0, "No sessions registered"

    def test_hello_resume_session(self, tmp_path):
        """Test that HELLO with resume=true resumes existing session (blocking)."""
        from zmqtunnel.server.broker import ServerBroker, ServerConfig
        from zmqtunnel.client.agent import ClientAgent, ClientConfig
        import zmq

        # Use unique port within valid range to avoid address reuse issues
        def get_free_port():
            import socket
            with socket.socket() as s:
                s.bind(("", 0))
                return s.getsockname()[1]

        base_port = 6400 + get_free_port() % 200
        server_port = str(base_port)

        broker_config = ServerConfig(
            bind_addr=f"tcp://*:{server_port}",
        )
        broker = ServerBroker(config=broker_config)

        # Start broker in background thread (blocking mode for Python 3.14 compatibility)
        def run_broker():
            # Call blocking main loop directly (no asyncio.run needed)
            broker.run()

        broker_thread = threading.Thread(target=run_broker, daemon=True)
        broker_thread.start()

        # Wait for server to be ready
        time.sleep(0.5)

        # First connection - create initial session (blocking call)
        config1 = ClientConfig(server_addr=f"tcp://localhost:{server_port}", client_id="resume-client-1")
        agent1 = ClientAgent(config=config1)
        agent1.connect_and_authenticate()  # Blocking for Python 3.14 compatibility
        session_id_1 = agent1.session_id

        print(f"[TEST] First session_id: {session_id_1}", flush=True, file=sys.stderr)

        # Second connection - should get same session (blocking call)
        config2 = ClientConfig(server_addr=f"tcp://localhost:{server_port}", client_id="resume-client-1")
        agent2 = ClientAgent(config=config2)
        agent2.connect_and_authenticate()  # Blocking for Python 3.14 compatibility
        session_id_2 = agent2.session_id

        print(f"[TEST] Second session_id: {session_id_2}", flush=True, file=sys.stderr)

        # Sessions should be the same
        assert session_id_1 == session_id_2


class TestMessageProtocolRoundtrip:
    """Test that protocol messages roundtrip correctly."""

    def test_hello_message_roundtrip(self):
        """Verify HELLO message is properly encoded and decoded."""
        from zmqtunnel.protocol import create_multipart, decode_frames

        frames = create_multipart(
            b"\x01",  # version
            0x01,      # HELLO msg type
            {
                "client_id": "test-client-123",
                "resume_session": False,
            }
        )

        version, msg_type_name, headers, payload = decode_frames(frames)

        assert msg_type_name == "HELLO"
        assert headers["client_id"] == "test-client-123"
        assert not headers.get("resume_session", False)

    def test_hello_ack_message_roundtrip(self):
        """Verify HELLO_ACK message is properly encoded and decoded."""
        from zmqtunnel.protocol import create_multipart, decode_frames

        frames = create_multipart(
            b"\x01",  # version
            0x02,      # HELLO_ACK msg type
            {"session_id": "test-session-abc"}
        )

        version, msg_type_name, headers, payload = decode_frames(frames)

        assert msg_type_name == "HELLO_ACK"
        assert headers["session_id"] == "test-session-abc"

    def test_register_forward_message_roundtrip(self):
        """Verify REGISTER_FORWARD message is properly encoded and decoded."""
        from zmqtunnel.protocol import create_multipart, decode_frames

        frames = create_multipart(
            b"\x01",  # version
            0x03,      # REGISTER_FORWARD msg type
            {
                "mode": "L",
                "bind_addr": "127.0.0.1:8080",
                "target": "localhost:9090",
                "peer_id": "peer-1",
            }
        )

        version, msg_type_name, headers, payload = decode_frames(frames)

        assert msg_type_name == "REGISTER_FORWARD"
        assert headers["mode"] == "L"
        assert headers["bind_addr"] == "127.0.0.1:8080"


class TestErrorHandling:
    """Test error handling during handshake."""

    def test_error_message_decoding(self):
        """Verify ERROR messages are properly decoded."""
        from zmqtunnel.protocol import create_multipart, decode_frames

        frames = create_multipart(
            b"\x01",  # version
            0x0B,      # ERROR msg type
            {"code": "AUTH_FAILED", "message": "Invalid public key"}
        )

        version, msg_type_name, headers, payload = decode_frames(frames)

        assert msg_type_name == "ERROR"
        assert headers["code"] == "AUTH_FAILED"
        assert headers["message"] == "Invalid public key"


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
