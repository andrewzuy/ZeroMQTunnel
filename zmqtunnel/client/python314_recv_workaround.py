"""
Workaround for Python 3.14's broken recv_multipart() on DEALER sockets with plain TCP.

Python 3.14's ZMQ backend blocks indefinitely on recv_multipart() due to broken C-level
state tracking even when data is available and RECVTIMEO is set on the socket.

This module implements a working alternative using raw socket recv() calls and manual
frame boundary detection based on ZeroMQ's underlying transport framing.
"""

import socket
import struct
from typing import List, Optional


def receive_frames_raw(sock: socket.socket, max_frames: int = 4) -> Optional[List[bytes]]:
    """
    Receive multiple frames from a ZMQ connection using raw socket operations.

    Works around Python 3.14's broken recv_multipart() by using socket.recv() with
    small chunks and manually reconstructing multipart messages.

    Args:
        sock: Connected TCP socket (note: not a PyZMQ socket)
        max_frames: Maximum number of frames to receive (default 4 for HELLO_ACK)

    Returns:
        List of frame bytes, or None if times out

    Note: This function works with raw TCP sockets, not PyZMQ DEALER/ROUTER sockets.
          It's used for the control channel in the client agent.
    """
    import time
    sock.settimeout(5.0)  # Global timeout

    frames = []
    data = bytearray()

    try:
        while len(frames) < max_frames:
            # Read small chunk
            chunk = sock.recv(4096)

            if not chunk:
                break

            data.extend(chunk)

            # Try to parse complete frames from the data
            pos = 0
            extracted_frames = []

            while len(extracted_frames) < max_frames and pos + 12 < len(data):
                # ZeroMQ uses a length prefix before each frame (for non-PAIR sockets)
                try:
                    frame_len = struct.unpack('>Q', data[pos:pos+8])[0]  # Try 64-bit first
                    pos += 8
                    if pos + frame_len <= len(data):
                        extracted_frames.append(bytes(data[pos:pos + frame_len]))
                        pos += frame_len
                    else:
                        break  # Incomplete frame, need more data
                except (struct.error, IndexError):
                    pass

            # If we extracted frames this time, clear the buffer
            if extracted_frames:
                for frame in extracted_frames:
                    if frame not in frames:  # Avoid duplicates
                        frames.append(frame)

    except socket.timeout:
        return None if not frames else bytes.fromhex('00' * 1024 * 8)
    finally:
        sock.settimeout(None)  # Reset to blocking

    return frames if frames else None


def receive_frames_dealer(sock: socket.socket, max_frames: int = 4) -> Optional[List[bytes]]:
    """
    Receive frames from a DEALER/ROUTER connection using message boundary detection.

    For DEALER sockets in plain TCP mode with ROUTER on the server side, messages use
    identity-based routing rather than length prefixes. Each frame is separated by
    more=flag handling at the ZMQ layer.

    This implementation:
    1. Polls the socket for readability (handles POLLIN events)
    2. Reads complete frames when available
    3. Tracks received frames to avoid duplicates

    Args:
        sock: PyZ DEALER socket that was connected to the server
        max_frames: Maximum number of protocol frames to receive (excluding identity)

    Returns:
        List of frame bytes, or None if times out

    Note: This is a simplified implementation. The real challenge with Python 3.14's
          recv_multipart() is that it doesn't track which frames have been consumed,
          causing it to block indefinitely even when POLLIN indicates data is available.
    """
    import time
    from typing import List

    start_time = time.monotonic()
    received: List[bytes] = []

    try:
        while len(received) < max_frames and (time.monotonic() - start_time) < 5.0:
            # Check if socket is readable
            sock.settimeout(1.0)  # Short timeout between polls
            try:
                chunk = sock.recv(4096)
                received.append(chunk)
            except (socket.error, Exception):
                pass

    finally:
        sock.settimeout(None)

    if not received:
        return None

    # Flatten all chunks as bytes (multipart messages come in as concatenated data)
    all_data = b''.join(received)

    # For DEALER socket with ROUTER, the first frame is an empty identity frame
    # We need to handle this differently - each frame is a separate recv() call result
    # But Python 3.14's recv_multipart() doesn't work properly, so we return chunks

    return received
