#!/usr/bin/env python3
"""Automatic reconnection and session management."""


class SelfHealingConfig:
    """Configuration for self-healing behavior."""
    
    RECONNECT_IVL = 1000            # Initial reconnect interval (ms)  
    RECONNECT_IVL_MAX = 30000       # Max reconnect interval (ms)
    MAX_RECONNECT_ATTEMPTS = None   
    HEARTBEAT_IVL = 5000            # Optional ZeroMQ heartbeat (ms)


class SessionManager:
    """Session management and cleanup."""
    
    def __init__(self):
        self.sessions: dict = {}  # session_id -> {agent_identity, tcp_socket, ...}
        self.session_close_callbacks = set()

    def register_session(self, session_id, agent_identity, tcp_socket=None):
        """Register a new session with the server."""
        
        self.sessions[session_id] = {
            'agent_identity': agent_identity,
            'tcp_socket': tcp_socket,
            'status': 'active',
            'created_at': datetime.now(),
        }

    def get_session(self, session_id) -> dict | None:
        """Get session by ID."""
        
        return self.sessions.get(session_id)

    def unregister_session(self, session_id):
        """Remove session from tracking."""
        
        if session_id in self.sessions:
            del self.sessions[session_id]

    def cleanup_dead_sessions(self, agent_identity):
        """Remove all sessions for dead agent."""
        
        stale = []
        
        for session_id, session_data in list(self.sessions.items()):
            if session_data['agent_identity'] == agent_identity:
                stale.append(session_id)
                
        for sid in stale:
            self.unregister_session(sid)
            
        return stale