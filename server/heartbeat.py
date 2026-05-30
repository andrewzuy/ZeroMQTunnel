from datetime import datetime, timedelta
import threading


class HeartbeatManager:
    """Applications-level heartbeat system for agent health monitoring."""
    
    HEARTBEAT_INTERVAL = 5.0           # seconds between heartbeats
    TIMEOUT_THRESHOLD = 15              # missed heartbeat threshold  
    CLEANUP_TIMEOUT = 30                # stale agent timeout (seconds)
    
    def __init__(self):
        self.agent_last_heartbeat: dict = {}
        self.agent_sessions: set = set()
        self._lock = threading.Lock()

    def heartbeat(self, agent_identity=None) -> dict:
        """Generate HEARTBEAT message from agent."""
        return {
            'type': 'HEARTBEAT',
            'agent_id': str(agent_identity) if isinstance(agent_identity, bytes) else agent_identity,
        }

    def heartbeat_ack(self, agent_identity=None, last_seen=None) -> dict:
        """Generate HEARTBEAT_ACK response from server."""
        return {
            'type': 'HEARTBEAT_ACK',
            'agent_id': str(agent_identity) if isinstance(agent_identity, bytes) else agent_identity,
        }

    def on_heartbeat(self, agent_identity):
        """Process incoming HEARTBEAT."""
        
        # Reset timeout counter for this agent
        self.agent_last_heartbeat[agent_identity] = datetime.now()

    def is_agent_alive(self, agent_identity, timeout_seconds=CLEANUP_TIMEOUT) -> bool:    
        """Check if agent has timed out based on last heartbeat."""
        
        with self._lock:
            if agent_identity not in self.agent_last_heartbeat:
                return False
            
            last_seen = self.agent_last_heartbeat[agent_identity]
           
            elapsed = (datetime.now() - last_seen).total_seconds()
            return elapsed < timeout_seconds

    def cleanup_dead_agent(self, agent_identity) -> list:
        """Forcefully close all streams and remove dead agent."""
        
        with self._lock:
            if agent_identity in self.agent_last_heartbeat:
                del self.agent_last_heartbeat[agent_identity]
            
            return list(self.agent_sessions.intersection({agent_identity}))
