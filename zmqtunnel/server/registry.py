"""
Session and tunnel registry for the server broker.
"""

import asyncio
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Set
import uuid


@dataclass
class TunnelSpec:
    """Specification for a forwarding tunnel."""
    tunnel_id: str
    mode: str  # 'L' or 'R'
    bind_addr: str
    target: str
    owner_client_id: str
    status: str = "active"


@dataclass
class ClientSession:
    """Client session tracked by the server."""
    client_id: str
    session_id: Optional[str]
    public_key: str
    assigned_id: Optional[str] = None
    tunnels: Dict[str, TunnelSpec] = field(default_factory=dict)


@dataclass
class Route:
    """A connection route between clients."""
    tunnel_id: str
    conn_id: str
    client_a_id: str
    client_b_id: str
    stream_id: Optional[str] = None  # ZMQ_STREAM identity if applicable


class Registry:
    """
    Server-side registry for sessions, tunnels, and routing tables.

    Maintains:
      - sessions: active client sessions with their state
      - tunnels: registered forwarding rules
      - routes: active connection routes between clients
      - acl: access control lists for authorization
    """

    def __init__(self):
        self._lock = asyncio.Lock()
        self.sessions: Dict[str, ClientSession] = {}
        self.tunnels: Dict[str, TunnelSpec] = {}
        self.routes: Dict[tuple[str, str], Route] = {}  # (client_a_id, conn_id) -> route

    async def register_session(
        self,
        client_id: str,
        session_id: Optional[str],
        public_key: str,
        assigned_id: Optional[str] = None,
    ) -> ClientSession:
        """Register a new client session or resume existing one."""
        async with self._lock:
            if session_id and session_id in self.sessions:
                # Resume existing session
                session = self.sessions[session_id]
                session.public_key = public_key
                if assigned_id:
                    session.assigned_id = assigned_id
                return session
            else:
                # New session
                new_session = ClientSession(
                    client_id=client_id,
                    session_id=session_id or str(uuid.uuid4()),
                    public_key=public_key,
                    assigned_id=assigned_id,
                )
                self.sessions[new_session.session_id] = new_session
                return new_session

    async def get_session(self, session_id: str) -> Optional[ClientSession]:
        """Get a client session by ID."""
        async with self._lock:
            return self.sessions.get(session_id)

    async def get_session_by_client_id(self, client_id: str) -> Optional[ClientSession]:
        """Find a session for a given client ID (may need to check all sessions)."""
        async with self._lock:
            for session in self.sessions.values():
                if session.client_id == client_id:
                    return session
            return None

    async def register_tunnel(
        self,
        tunnel_spec: TunnelSpec,
    ) -> bool:
        """Register a forwarding tunnel rule."""
        async with self._lock:
            existing = self.tunnels.get(tunnel_spec.tunnel_id)
            if existing and existing.status != "deleted":
                return False  # Already exists

            self.tunnels[tunnel_spec.tunnel_id] = tunnel_spec
            return True

    async def get_tunnel(self, tunnel_id: str) -> Optional[TunnelSpec]:
        """Get a tunnel specification."""
        async with self._lock:
            return self.tunnels.get(tunnel_id)

    async def delete_tunnel(self, tunnel_id: str) -> None:
        """Mark a tunnel for deletion."""
        async with self._lock:
            if tunnel_id in self.tunnels:
                self.tunnels[tunnel_id].status = "deleted"

    async def get_tunnels_for_client(self, client_id: str) -> List[TunnelSpec]:
        """Get all active tunnels owned by a client."""
        async with self._lock:
            return [
                tunnel for tunnel in self.tunnels.values()
                if tunnel.owner_client_id == client_id and tunnel.status == "active"
            ]

    async def add_route(
        self,
        tunnel_id: str,
        conn_id: str,
        client_a_id: str,
        client_b_id: str,
        stream_id: Optional[str] = None,
    ) -> bool:
        """Add a connection route between clients."""
        async with self._lock:
            key = (client_a_id, conn_id)
            if key in self.routes:
                return False  # Route already exists

            self.routes[key] = Route(
                tunnel_id=tunnel_id,
                conn_id=conn_id,
                client_a_id=client_a_id,
                client_b_id=client_b_id,
                stream_id=stream_id,
            )
            return True

    async def get_route_target(self, client_id: str, conn_id: str) -> Optional[str]:
        """Get the target client ID for a given connection."""
        async with self._lock:
            key = (client_id, conn_id)
            route = self.routes.get(key)
            if route:
                return route.client_b_id
            return None

    async def remove_route(self, client_id: str, conn_id: str) -> None:
        """Remove a connection route."""
        async with self._lock:
            key = (client_id, conn_id)
            if key in self.routes:
                del self.routes[key]

    async def list_active_routes(self) -> List[Route]:
        """List all active routes."""
        async with self._lock:
            return list(self.routes.values())

    async def cleanup_deleted_tunnels(self) -> None:
        """Clean up references to deleted tunnels."""
        async with self._lock:
            tunnels_to_remove = []
            for key, route in self.routes.items():
                tunnel_id = next(
                    (tunnel.tunnel_id for tunnel in self.tunnels.values()
                     if tunnel.tunnel_id == route.tunnel_id), None
                )
                if tunnel_id and next((tunnel for tunnel in self.tunnels.values()
                                     if tunnel.tunnel_id == route.tunnel_id)).status == "deleted":
                    tunnels_to_remove.append((key, route))

            for key, _ in tunnels_to_remove:
                del self.routes[key]

    def stats(self) -> dict:
        """Get registry statistics."""
        return {
            "active_sessions": len([s for s in self.sessions.values() if s.session_id]),
            "tunnels": len([t for t in self.tunnels.values() if t.status == "active"]),
            "routes": len([r for r in self.routes.values() if r.client_a_id and r.client_b_id]),
        }
