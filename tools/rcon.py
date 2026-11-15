"""Minimal RCON client for the local test server. Reads MC_RCON_* from the repo .env.

Usage: python3 tools/rcon.py "<command>" ["<command>" ...]
Prints each command's response. The password is never printed.
"""

import os
import socket
import struct
import sys
from pathlib import Path


def load_env() -> dict:
    env = {}
    path = Path(__file__).resolve().parent.parent / ".env"
    for line in path.read_text().splitlines():
        if "=" in line and not line.lstrip().startswith("#"):
            k, v = line.split("=", 1)
            env[k.strip()] = v.strip()
    return env


class Rcon:
    def __init__(self, host: str, port: int, password: str):
        self.sock = socket.create_connection((host, port), timeout=10)
        self.req = 0
        if self._send(3, password)[0] == -1:
            raise SystemExit("rcon auth failed")

    def _recv_exact(self, n: int) -> bytes:
        buf = b""
        while len(buf) < n:
            chunk = self.sock.recv(n - len(buf))
            if not chunk:
                raise ConnectionError("rcon closed")
            buf += chunk
        return buf

    def _read_packet(self):
        (size,) = struct.unpack("<i", self._recv_exact(4))
        body = self._recv_exact(size)
        rid, _kind = struct.unpack("<ii", body[:8])
        return rid, body[8:-2].decode("utf-8", "replace")

    def _send(self, kind: int, payload: str):
        self.req += 1
        data = struct.pack("<ii", self.req, kind) + payload.encode() + b"\x00\x00"
        self.sock.sendall(struct.pack("<i", len(data)) + data)
        # The server splits a long response into several packets; the tail of an earlier
        # one arrives first and would answer this command. Skip packets for older requests.
        while True:
            rid, body = self._read_packet()
            if rid == self.req or rid == -1:
                return rid, body

    def cmd(self, command: str) -> str:
        return self._send(2, command)[1]


def connect() -> Rcon:
    env = {**load_env(), **os.environ}
    return Rcon(
        env.get("MC_RCON_HOST", "127.0.0.1"),
        int(env.get("MC_RCON_PORT", "25575")),
        env["MC_RCON_PASSWORD"],
    )


if __name__ == "__main__":
    r = connect()
    for c in sys.argv[1:]:
        print(r.cmd(c))
