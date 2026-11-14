"""RCON and world-probe helpers for the staging server (tools/stage/end_tower.sh).

The password is read from <stage dir>/.rcon and never printed.
"""

import math
import os
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from rcon import Rcon  # noqa: E402

STAGE = Path(os.environ.get("STAGE_DIR", Path.home() / "minecraft" / "stage-hardcore"))
END = "minecraft:the_end"
NUM = r"(-?\d+(?:\.\d+)?)[dfb]?"


def connect() -> Rcon:
    host = os.environ.get("STAGE_RCON_HOST", "127.0.0.1")
    port = int(os.environ.get("STAGE_RCON_PORT", "25576"))
    return Rcon(host, port, (STAGE / ".rcon").read_text().strip())


def in_end(r: Rcon, command: str) -> str:
    return r.cmd(f"execute in {END} run {command}")


def passed(reply: str) -> bool:
    return "passed" in reply


def block_is(r: Rcon, x: int, y: int, z: int, block: str) -> bool:
    return passed(r.cmd(f"execute in {END} if block {x} {y} {z} minecraft:{block}"))


def entity_is(r: Rcon, selector: str) -> bool:
    return passed(r.cmd(f"execute in {END} if entity {selector}"))


def floats(reply: str, count: int) -> list[float] | None:
    """The first `count` numbers inside the [...] of a `data get` reply."""
    m = re.search(r"\[([^\]]*)\]", reply)
    if not m:
        return None
    vals = [float(v) for v in re.findall(NUM, m.group(1))]
    return vals[:count] if len(vals) >= count else None


def get_pos(r: Rcon, selector: str) -> list[float] | None:
    return floats(r.cmd(f"data get entity {selector} Pos"), 3)


def get_rot(r: Rcon, selector: str) -> list[float] | None:
    return floats(r.cmd(f"data get entity {selector} Rotation"), 2)


def dimension_of(r: Rcon, player: str) -> str:
    m = re.search(r'"([^"]+)"', r.cmd(f"data get entity {player} Dimension"))
    return m.group(1) if m else ""


def look_angles(eye: tuple, target: tuple) -> tuple[float, float]:
    """Minecraft yaw and pitch (degrees) for looking from `eye` at `target`."""
    dx, dy, dz = (target[i] - eye[i] for i in range(3))
    yaw = math.degrees(math.atan2(-dx, dz))
    pitch = -math.degrees(math.atan2(dy, math.hypot(dx, dz)))
    return round(yaw, 2), round(pitch, 2)


def gaze_error(eye: tuple, rot: list[float], target: tuple) -> float:
    """Angle in degrees between a facing (yaw, pitch) and the direction eye -> target."""
    yaw, pitch = math.radians(rot[0]), math.radians(rot[1])
    f = (-math.sin(yaw) * math.cos(pitch), -math.sin(pitch), math.cos(yaw) * math.cos(pitch))
    d = [target[i] - eye[i] for i in range(3)]
    n = math.sqrt(sum(c * c for c in d)) or 1.0
    dot = sum(f[i] * d[i] / n for i in range(3))
    return math.degrees(math.acos(max(-1.0, min(1.0, dot))))


def pillar_tops(r: Rcon) -> list[tuple[int, int, int]]:
    """(x, z, top obsidian y) of the ten end pillars, found by block tests, not by seed."""
    tops = []
    for i in range(10):
        x = math.floor(42 * math.cos(2 * math.pi * i / 10))
        z = math.floor(42 * math.sin(2 * math.pi * i / 10))
        lo, hi = 30, 140
        while lo < hi:  # obsidian is solid from the ground up to the top, so bisect
            mid = (lo + hi + 1) // 2
            if block_is(r, x, mid, z, "obsidian"):
                lo = mid
            else:
                hi = mid - 1
        if lo >= 76:  # spikes are 76..103 high; anything lower is not a pillar column
            tops.append((x, z, lo))
    return tops
