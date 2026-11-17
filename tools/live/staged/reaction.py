"""Reflex reaction time on the live server: from the tick a stimulus is in the body's sensed
state to the tick the first motor command of the response leaves.

Usage: python3 reaction.py [trials] [cases, comma-separated]

Each case stages one stimulus with one RCON command and reads the bot trace
(MOTOR_TRACE=1). Per trial:
  seen  first `T` line (or `B` watch line) after the command whose sensed state holds the
        stimulus: its tick and its wall-clock ms
  resp  first `T` line at or after `seen` on which the reflex owns its channel
  body  resp tick - seen tick, in ticks (x 50 ms), plus the `A` line of the response tick:
        microseconds from the start of sensing to the last packet handed to the connection
  link  seen ms - the ms the RCON command was sent (and - the ms its reply was read): server
        tick, loopback and the wait for the body's next tick, outside the body's control
The outcome is read back on the server (position, health, food, offhand).

Cases: fall (R6, teleported 30 up with a water bucket), edge (R5, the floor ahead removed
while walking), creeper (D1, a creeper summoned 5 blocks away), archer (D3, a skeleton
summoned 10 away: first command, and shield raise after the draw is seen), eat (H1, food
driven to the floor by a hunger effect), totem (R11, damaged to 5 HP with a totem carried),
retreat (H2, damaged to 7 HP with a zombie 6 away).
"""
import json, re, statistics, sys, time
from arena import NAME, S, bot_start, bot_stop, rc, send, now_ms

X, Y, Z = 1000, 150, 1000
KV = re.compile(r"(\w+)=(\S+)")


class Tail:
    """Reads the bot log from where the trial began."""

    def __init__(self):
        self.path = f"{S}/bot.log"
        with open(self.path, "rb") as f:
            f.seek(0, 2)
            self.at = f.tell()

    def lines(self):
        with open(self.path, "rb") as f:
            f.seek(self.at)
            return f.read().decode(errors="replace").splitlines()

    def parsed(self):
        ticks, watches, acted = [], [], {}
        for l in self.lines():
            if l.startswith("T "):
                d = dict(KV.findall(l))
                d["tick"] = int(l.split()[1])
                d["ms"] = int(d["ms"])
                d["pos"] = tuple(float(v) for v in d["pos"].split(","))
                ticks.append(d)
            elif l.startswith("B "):
                p = l.split()
                watches.append(dict(tick=int(p[1]), cell=tuple(int(v) for v in p[2:5]), block=p[5]))
            elif l.startswith("A "):
                p = l.split()
                acted[int(p[1])] = int(p[2].split("=")[1])
        return ticks, watches, acted


def num(path):
    m = re.search(r"(-?[\d.]+)[fdsb]?$", rc(f"data get entity {NAME} {path}").strip())
    return float(m.group(1)) if m else None


def server_pos():
    v = re.findall(r"(-?[\d.]+)d", rc(f"data get entity {NAME} Pos"))
    return tuple(float(a) for a in v[:3]) if len(v) >= 3 else None


def prep(items=(), at=(X + 0.5, Y, Z + 0.5)):
    # A killed mob stays in the client's entity list for its 20-tick death animation.
    rc(f"kill @e[type=!player,x={X},y={Y},z={Z},distance=..60]")
    time.sleep(1.3)
    rc(f"fill {X - 10} {Y - 1} {Z - 12} {X + 10} {Y - 1} {Z + 12} stone",
       f"fill {X - 10} {Y} {Z - 12} {X + 10} {Y + 8} {Z + 12} air",
       f"effect clear {NAME}", f"gamemode survival {NAME}", f"clear {NAME}",
       f"effect give {NAME} instant_health 1 5", f"effect give {NAME} saturation 1 20",
       *[f"item replace entity {NAME} {s} with {i} {n}" for s, i, n in items],
       f"tp {NAME} {at[0]} {at[1]} {at[2]} 0 0")
    send("goal-end")
    send("unwatch")
    time.sleep(2.0)


def stimulate(cmd):
    """Sends the stimulus; returns (ms sent, ms the reply was read)."""
    t0 = now_ms()
    rc(cmd)
    return t0, now_ms()


def result(tail, sent, ack, seen_fn, resp_fn, wait=1.5, watch_cell=None):
    """`seen_fn(row)`/`resp_fn(row)` pick the stimulus and response ticks from `T` rows; with
    `watch_cell` the stimulus is the `B` line that shows that cell as air."""
    time.sleep(wait)
    ticks, watches, acted = tail.parsed()
    ticks = [r for r in ticks if r["ms"] >= sent]
    if watch_cell:
        w = next((b for b in watches if b["cell"] == watch_cell and b["block"] == "air"), None)
        seen = next((r for r in ticks if w and r["tick"] == w["tick"]), None)
    else:
        seen = next((r for r in ticks if seen_fn(r)), None)
    if not seen:
        return dict(ok=False, why="stimulus not seen")
    resp = next((r for r in ticks if r["tick"] >= seen["tick"] and resp_fn(r)), None)
    if not resp:
        return dict(ok=False, why="no response", seen_tick=seen["tick"])
    return dict(ok=True, seen_tick=seen["tick"], resp_tick=resp["tick"], body_ticks=resp["tick"] - seen["tick"],
                body_ms=resp["ms"] - seen["ms"], act_us=acted.get(resp["tick"]),
                link_sent_ms=seen["ms"] - sent, link_ack_ms=seen["ms"] - ack, rcon_ms=ack - sent)


def owns(rid, *channels):
    return lambda r: any(r.get(c) == rid for c in channels)


def fall():
    prep(items=[("hotbar.0", "diamond_sword", 1), ("hotbar.8", "water_bucket", 1)])
    hp0 = num("Health")
    tail = Tail()
    sent, ack = stimulate(f"tp {NAME} {X + 0.5} {Y + 30} {Z + 0.5} 0 0")
    r = result(tail, sent, ack, lambda r: r["ground"] == "0" and r["pos"][1] > Y + 10,
               owns("R6", "gaze", "main"), wait=4.5)
    r["hp_lost"] = round(hp0 - (num("Health") or 0), 1)
    return r


def edge():
    gap = Z + 2
    prep(items=[("hotbar.0", "diamond_sword", 1)], at=(X + 0.5, Y, Z - 8.5))
    send(f"watch {X} {Y - 1} {gap}")
    time.sleep(0.3)
    tail = Tail()
    send(f"goal {X + 0.5} {Y} {Z + 10.5}")
    end = time.time() + 15
    z = None
    while time.time() < end:
        ticks, _, _ = tail.parsed()
        if ticks and ticks[-1]["pos"][2] >= gap - 1.2:
            z = ticks[-1]["pos"][2]
            break
        time.sleep(0.005)
    if z is None:
        return dict(ok=False, why="never walked")
    sent, ack = stimulate(f"fill {X - 2} {Y - 1} {gap} {X + 2} {Y - 1} {gap + 4} air")
    time.sleep(2.5)
    # A veto leaves the goal as the owner of the legs, so the response is read from the
    # motion: the tick before the first one whose forward speed has dropped (a tick's
    # `vel` is the result of the tick before it). The `edge_guard` event, printed the
    # first time only, names the same tick.
    ticks, _, _ = tail.parsed()
    vz = {t["tick"]: float(t["vel"].split(",")[2]) for t in ticks}
    halted = {t for t in vz if t - 1 in vz and vz[t] < 0.9 * vz[t - 1]}
    r = result(tail, sent, ack, None, lambda r: r["tick"] + 1 in halted, wait=0.0, watch_cell=(X, Y - 1, gap))
    ev = [int(l.split()[1]) for l in tail.lines() if l.startswith("E ") and "edge_guard  R5" in l]
    r["event_tick"] = ev[0] if ev else None
    p = server_pos()
    at = next((t for t in ticks if t["tick"] == r.get("seen_tick")), None)
    r.update(fell=bool(p and p[1] < Y - 0.5), server_pos=p, edge_dist_seen=round(gap - at["pos"][2], 2) if at else None,
             edge_dist_end=round(gap - p[2], 2) if p else None)
    send("goal-end")
    return r


def creeper():
    prep()
    tail = Tail()
    sent, ack = stimulate(f"summon creeper {X + 0.5} {Y} {Z + 5.5} {{NoAI:1b,PersistenceRequired:1b}}")
    r = result(tail, sent, ack, lambda r: r["near"].startswith("creeper"), owns("D1", "legs"), wait=3.0)
    ticks, _, _ = tail.parsed()
    at = next((t for t in ticks if t["tick"] == r.get("seen_tick")), None)
    moved = next((t for t in ticks if at and t["tick"] > at["tick"] and t["pos"] != at["pos"]), None)
    # The legs command steers the next physics step: the first changed position.
    r["move_ticks"] = moved["tick"] - at["tick"] if moved else None
    p = server_pos()
    r["dist_after_3s"] = round(((p[0] - X - 0.5) ** 2 + (p[2] - Z - 5.5) ** 2) ** 0.5, 2) if p else None
    return r


def archer():
    prep(items=[("hotbar.0", "diamond_sword", 1), ("weapon.offhand", "shield", 1)])
    hp0 = num("Health")
    tail = Tail()
    sent, ack = stimulate(f'summon skeleton {X + 0.5} {Y} {Z + 10.5} {{PersistenceRequired:1b,equipment:{{'
                          f'mainhand:{{id:"minecraft:bow",count:1}},head:{{id:"minecraft:leather_helmet",count:1}}}}}}')
    r = result(tail, sent, ack, lambda r: r["near"].startswith("skeleton"),
               owns("D3", "legs", "gaze", "main", "off"), wait=8.0)
    ticks, _, acted = tail.parsed()
    draw = next((t for t in ticks if t["ms"] >= sent and ":draw" in t["near"]), None)
    up = next((t for t in ticks if draw and t["tick"] >= draw["tick"] and t.get("off") == "D3"), None)
    r.update(draw_tick=draw["tick"] if draw else None, shield_ticks=up["tick"] - draw["tick"] if up else None,
             draw_at=draw["near"] if draw else None, hp_lost=round(hp0 - (num("Health") or 0), 1))
    return r


def eat():
    prep(items=[("hotbar.0", "diamond_sword", 1), ("hotbar.3", "bread", 8)])
    tail = Tail()
    sent, ack = stimulate(f"effect give {NAME} hunger 30 255 true")
    time.sleep(6.0)
    r = result(tail, sent, ack, lambda r: int(r["food"]) <= 14, owns("H1", "main", "inv"), wait=0.0)
    rc(f"effect clear {NAME} hunger")
    time.sleep(2.5)
    r["server_food_after"] = num("foodLevel")
    r["bread_left"] = rc(f"clear {NAME} minecraft:bread 0")
    for k in ("link_sent_ms", "link_ack_ms"):
        r.pop(k, None)  # the food bar falls over seconds; the command time says nothing
    return r


def totem():
    prep(items=[("hotbar.0", "diamond_sword", 1), ("hotbar.5", "totem_of_undying", 1)])
    tail = Tail()
    sent, ack = stimulate(f"damage {NAME} 15 minecraft:generic")
    r = result(tail, sent, ack, lambda r: float(r["hp"]) <= 6.0, owns("R11", "inv"), wait=1.5)
    r["offhand"] = "totem" in rc(f"data get entity {NAME} equipment.offhand.id")
    return r


def retreat():
    prep()
    rc(f"summon zombie {X + 0.5} {Y} {Z + 6.5} {{NoAI:1b,PersistenceRequired:1b}}")
    time.sleep(1.0)
    tail = Tail()
    p0 = server_pos()
    sent, ack = stimulate(f"damage {NAME} 15 minecraft:generic")
    r = result(tail, sent, ack, lambda r: float(r["hp"]) < 8.0, owns("H2", "legs"), wait=1.2)
    p = server_pos()
    d = lambda q: round(((q[0] - X - 0.5) ** 2 + (q[2] - Z - 6.5) ** 2) ** 0.5, 2) if q else None
    r.update(dist_before=d(p0), dist_after=d(p))
    return r


CASES = dict(fall=fall, edge=edge, creeper=creeper, archer=archer, eat=eat, totem=totem, retreat=retreat)


def pct(v, q):
    v = sorted(v)
    return v[min(len(v) - 1, max(0, round(q * (len(v) - 1))))]


def summary(name, rs):
    ok = [r for r in rs if r.get("ok")]
    out = dict(case=name, n=len(rs), measured=len(ok))
    for key in ("body_ticks", "act_us", "link_sent_ms", "link_ack_ms", "rcon_ms", "shield_ticks", "move_ticks",
                "edge_dist_seen", "edge_dist_end"):
        v = [r[key] for r in ok if r.get(key) is not None]
        if v:
            out[key] = dict(n=len(v), median=statistics.median(v), p90=pct(v, 0.9), max=max(v))
    return out


if __name__ == "__main__":
    trials = int(sys.argv[1]) if len(sys.argv) > 1 else 20
    names = sys.argv[2].split(",") if len(sys.argv) > 2 else list(CASES)
    rc("gamerule spawn_mobs false", "forceload add 944 944 1056 1056")
    bot_start()
    try:
        for name in names:
            rs = []
            for i in range(trials):
                r = CASES[name]()
                r.update(case=name, trial=i + 1)
                rs.append(r)
                print(json.dumps(r), flush=True)
            print("SUMMARY " + json.dumps(summary(name, rs)), flush=True)
    finally:
        rc(f"kill @e[type=!player,x={X},y={Y},z={Z},distance=..60]", f"effect clear {NAME}")
        bot_stop()
