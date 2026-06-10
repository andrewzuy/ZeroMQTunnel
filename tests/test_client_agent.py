"""Tests for the client agent."""

import pytest


class TestClientAgent:
    """Test client agent functionality."""

    @pytest.fixture
    def agent(self):
        from zmqtunnel.client.agent import ClientConfig, ClientAgent
        return ClientAgent(
            ClientConfig(
                server_addr="tcp://localhost:5555",
                server_key_path="/tmp/server.key",
                client_key_path="/tmp/client.key",
                local_bind="tcp://*:8080",
            )
        )

    def test_agent_initialization(self, agent):
        """Test that agent initializes correctly."""
        assert agent.config.server_addr == "tcp://localhost:5555"
        assert agent.config.local_bind == "tcp://*:8080"
        assert len(agent.connections) == 0

    def test_agent_initialization_remote_mode(self):
        from zmqtunnel.client.agent import ClientConfig, ClientAgent

        config = ClientConfig(
            server_addr="tcp://localhost:5555",
            local_bind=None,
            remote_bind="tcp://*:9090",
        )
        agent = ClientAgent(config)

        assert agent.config.remote_bind == "tcp://*:9090"


class TestServerBroker:
    """Test server broker functionality."""

    @pytest.fixture
    def registry(self):
        from zmqtunnel.server.registry import Registry
        return Registry()

    async def test_registry_session_registration(self, registry):
        """Test client session registration."""
        session = await registry.register_session(
            client_id="client123",
            session_id=None,
            public_key="pubkey_test",
            assigned_id=None,
        )
        assert session.client_id == "client123"
        assert "session_id" in session.__dict__

    async def test_registry_tunnel_registration(self, registry):
        """Test tunnel registration."""
        from zmqtunnel.protocol import TunnelSpec

        spec = TunnelSpec(
            tunnel_id="L_tcp___8080_target_http___target_80",
            mode="L",
            bind_addr="tcp://*:8080",
            target="http://target:80",
            owner_client_id="client123",
        )

        result = await registry.register_tunnel(spec)
        assert result is True

    async def test_registry_get_tunnel(self, registry):
        """Test getting a tunnel by ID."""
        from zmqtunnel.protocol import TunnelSpec

        spec = TunnelSpec(
            tunnel_id="test_tunnel",
            mode="L",
            bind_addr="tcp://*:8080",
            target="http://target:80",
            owner_client_id="client123",
        )
        await registry.register_tunnel(spec)

        tunnel = await registry.get_tunnel("test_tunnel")
        assert tunnel is not None
        assert tunnel.tunnel_id == "test_tunnel"

    async def test_registry_get_tunnels_for_client(self, registry):
        """Test getting tunnels for a client."""
        from zmqtunnel.protocol import TunnelSpec

        await registry.register_tunnel(TunnelSpec(
            tunnel_id="t1", mode="L", bind_addr="tcp://*:8080",
            target="http://target:80", owner_client_id="client123"
        ))
        await registry.register_tunnel(TunnelSpec(
            tunnel_id="t2", mode="R", bind_addr="tcp://*:9090",
            target="http://server:22", owner_client_id="client456"
        ))

        tunnels = await registry.get_tunnels_for_client("client123")
        assert len(tunnels) == 1
        assert tunnels[0].tunnel_id == "t1"


class TestStreamBridge:
    """Test stream bridge functionality."""

    async def test_stream_identity_map(self):
        from zmqtunnel.stream_bridge import StreamIdentityMap

        map = StreamIdentityMap()

        await map.register("stream_1", "conn_aaa", "tcp://localhost:80")
        assert map.get_target("stream_1") == "tcp://localhost:80"

        stream_id2 = await map.unregister("conn_aaa")
        assert stream_id2 == "stream_1"

    def test_event_names(self):
        from zmqtunnel.stream_bridge import EventNames

        assert EventNames.EVENT_CONNECT == b"EVENT_CONNECT"
        assert EventNames.EVENT_DISCONNECT == b"EVENT_DISCONNECT"
