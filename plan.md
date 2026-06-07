# Development Plan: Go TCP Tunnel (PoC) using ZeroMQ

## Project Overview
The goal is to build a lightweight, robust tool in Go that implements Local and Remote port forwarding similar to SSH's functionality. The tool will use **ZeroMQ** as the transport layer between clients and the intermediate server to ensure connection resilience.

**Constraints:** No encryption, no load balancing, minimal dependencies.

## Architecture Overview
1.  **Transport Layer (ZeroMQ)**: Uses ZeroMQ to bridge communication between nodes. This provides inherent reliability features like automatic reconnection.
2.  **Framing Logic**: Since ZeroMQ is message-oriented and the source/target are TCP streams, a framing layer will be used to segment the stream into packets for transport over ZeroMQ.
3.  **Server Logic**: 
    - Listens on a "Gate" port for client connections (Standard TCP or ZMQ).
    - Handles multiplexing of multiple client connections via ZMQ's `ROUTER/DEALER` patterns.
4.  **Client Logic**:
    - Listens on local ports (Local Forwarding).
    - Communicates with the server using ZeroMQ to establish a persistent tunnel.

## Implementation Phases

### Phase 1: Core Tunnel Engine & Framing
- Implement a mechanism to "chunk" TCP data into segments that can be wrapped as ZeroMQ messages.
- Create the bridge logic to route these chunks between two endpoints.
- Ensure sequence integrity if necessary, though standard ZMQ DEALER/ROUTER handles point-to-point reliably.

### Phase 2: Server Implementation (ZeroMQ Backend)
- Implement a listener for the **Gateway** using ZeroMQ (e.g., `ZMQ_ROUTER`).
- Manage multiple concurrent client tunnels using a "Broker" pattern to route messages to the correct local session.

### Phase 3: Client & Local Forwarding
- Implement local port listeners on the client side.
- Establish a connection to the server's ZeroMQ gateway.
- Map the local TCP socket's input to the ZeroMQ outbound stream.

### Phase 4: Remote Forwarding Implementation
- Implement listener functionality on the server for ports designated for "Remote" use.
- Route incoming traffic from these ports through the ZeroMQ mesh back to the client machine.

### Phase 5: CLI & Configuration
- Implement a command-line interface (e.g., using `flag` or `cobra`).
- Support flags for:
    - `-listen`: Local port to listen on.
    - `-remote`: Remote port to bridge to.
    - `-gateway`: Address/Port of the intermediary server.
- Add basic signal handling to exit cleanly.

## Success Criteria (PoC)
1.  **Local Forwarding**: A user connects to `localhost:X`, and the traffic is encapsulated in ZeroMQ messages, delivered via the server, and extracted at the destination.
2.  **Remote Forwarding**: External users connect to `server_ip:Y`, and the traffic flows back through the ZeroMQ tunnel to the client machine.
3.  **Reliability**: The tool successfully handles reconnection of the underlying transport when the connection is interrupted (leveraging ZMQ's retry logic).
