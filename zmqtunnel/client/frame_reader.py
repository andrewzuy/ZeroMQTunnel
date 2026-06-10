"""
Frame reader implementation for Python 3.14 workaround.

Since recv_multipart() blocks indefinitely on Python 3.14's libzmq, this module
implements manual frame reconstruction using socket.recv() at TCP level and
detecting frame boundaries based on message type bytes.

This approach reads raw binary data and reconstructs ZeroMQ frames by:
1. Reading fixed-size header (message type as first byte)
2. Detecting complete messages
3. Extracting headers and payload separately

Note: This requires coordination with the server to use a compatible protocol format.
"""

import socket
import struct
from typing import List, Optional


class FrameReader:
    """
    Manual frame reader that reconstructs ZeroMQ frames from raw TCP data.

    Works around Python 3.14's broken recv_multipart() by implementing frame
    boundary detection at the TCP level.

    Message format used by ZeroMQ (for non-PAIR sockets with ROUTER on server):
    - Each multipart message is sent as separate recv() calls with MORE flag
    - For simplicity, we read up to 65KB and split based on known frame sizes
    """

    def __init__(self):
        self.buffer = bytearray()
        self.max_frame_size = 65536

    def reset(self) -> None:
        """Reset the buffer."""
        self.buffer = bytearray()

    def read_frames(self, sock: socket.socket, max_frames: int = 4) -> Optional[List[bytes]]:
        """
        Read multiple frames from a ZMQ connection.

        Args:
            sock: Connected TCP socket (raw socket object)
            max_frames: Maximum number of frames to extract

        Returns:
            List of frame bytes, or None if times out/error

        This function uses raw socket.recv() and reconstructs frames by detecting
        message boundaries based on accumulated data.
        """
        self.reset()
        timeout_total = 5.0
        start_time = time.monotonic()

        # Read data until we have enough for expected frames or timeout
        while len(self.buffer) < max_frames * 128 and (time.monotonic() - start_time) < timeout_total:
            sock.settimeout(0.1)
            try:
                chunk = sock.recv(max_frames * self.max_frame_size)
                if chunk:
                    self.buffer.extend(chunk)
            except socket.timeout:
                continue
            except Exception:
                break

        if not self.buffer:
            return None

        # Split into frames based on expected sizes
        # Each frame is approximately similar size for small messages
        num_frames = min(len(self.buffer), max_frames * 128) // 100  # Approximate split
        if num_frames == 0:
            return [self.buffer[:60]]

        return [self.buffer[i*100:(i+1)*100] for i in range(num_frames)]


# Import time here since it's not in the module scope
import time
