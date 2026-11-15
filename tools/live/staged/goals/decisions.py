"""The decision log live, over the real tool path (brain-link-mcp on stdio). A fixed kit, a
craft chain and one call of every kind that changes behaviour; then every line of the log
is read back and checked against the server: the position and the inventory in the record
against RCON at the moment of the call, the running goal against the goal just set, and the
inventory after the craft against the server's (the state must be this tick's, not an old
observe). Prints the cost per record the bot measured (`DECISION t= n= us=` lines).

Usage: python3 decisions.py     (needs the bot's bridge socket free)
"""
import json, statistics, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mcp_drive import Mcp  # noqa: E402
from crafting import clear_site, site  # noqa: E402
from kit import *  # noqa: E402,F401

KIT = [("hotbar.1", "oak_log", 2), ("hotbar.2", "cobblestone", 3), ("hotbar.3", "bread", 5)]
LOG = Path(f"{S}/bot.decisions.jsonl")


def server_inventory(names):
    return {n: c for n in names if (c := count_item(NAME, n)) and isinstance(c, int)}


def main():
    site()
    LOG.unlink(missing_ok=True)
    start()
    checks, calls = [], []
    try:
        prep(items=KIT, comp_at=(CX - 3.5, Y + 1, CZ - 3.5))
        m = Mcp()
        m.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                             "clientInfo": {"name": "decisions", "version": "0"}})
        m.p.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
        m.p.stdin.flush()

        def call(name, args):
            at = dict(pos=pos(), inv=server_inventory(["oak_log", "cobblestone", "bread", "stone_pickaxe"]))
            text, err, _ = m.tool(name, args)
            calls.append(dict(name=name, args=args, reply=text, err=err, **at))

        call("set_mode", {"mode": "build"})
        call("set_goal", {"goal": "craft", "params": {"kind": "stone_pickaxe", "count": 1}, "priority": 0.7})
        time.sleep(1.0)
        call("set_gain", {"gain": "caution", "value": 0.8})
        call("inhibit", {"reflex": "R5", "scope": {"kind": "here", "radius": 6}, "ttl_s": 30})
        call("arm", {"trigger": {"kind": "chat_matches", "pattern": "ping"},
                     "response": {"kind": "say", "text": "pong"}})
        call("say", {"text": "making a pickaxe"})
        end = time.time() + 90
        while time.time() < end and count_item(NAME, "stone_pickaxe") != 1:
            time.sleep(1)
        time.sleep(2)
        call("set_goal", {"goal": "place", "params": {"kind": "cobblestone", "pos": [CX + 3, Y + 1, CZ],
                                                      "to": [CX + 3, Y + 2, CZ], "replace": True}})
        call("set_goal", {"goal": "end"})
        m.p.stdin.close()
        m.p.wait(timeout=5)
        recs = [json.loads(l) for l in LOG.read_text().splitlines()]
        ops = [r["call"]["op"] for r in recs]
        checks.append(("one line per call", ops == [c["name"] for c in calls], ops))
        ticks = [r["tick"] for r in recs]
        checks.append(("ticks rise", ticks == sorted(ticks), ticks))
        for r, c in zip(recs, calls):
            st, sent = r["state"], dict(c["args"])
            got = {k: v for k, v in r["call"].items() if k != "op" and v != {}}
            checks.append((f"{c['name']} parameters whole", got == sent, got))
            checks.append((f"{c['name']} position within 1 block of the server",
                           dist(tuple(st["pos"]), c["pos"]) is not None and dist(tuple(st["pos"]), c["pos"]) <= 1.0,
                           (st["pos"], c["pos"])))
            if c is not calls[2]:  # the craft is consuming items between the query and the call
                same = {k: v for k, v in st["inventory"].items() if k in c["inv"] or k in
                        ("oak_log", "cobblestone", "bread", "stone_pickaxe")} == c["inv"]
                checks.append((f"{c['name']} inventory equals the server's", same, (st["inventory"], c["inv"])))
        run = recs[2]["state"]["goals"]["running"] or {}
        checks.append(("the craft goal shows as running with its plan step at the next call",
                       run.get("spec") == "craft stone_pickaxe 1" and run.get("id") == recs[1]["result"]["id"]
                       and run.get("note", "").startswith("plan: craft "), run))
        checks.append(("inventory before the craft has no pickaxe, after it one",
                       "stone_pickaxe" not in recs[1]["state"]["inventory"]
                       and recs[6]["state"]["inventory"].get("stone_pickaxe") == 1, recs[6]["state"]["inventory"]))
        checks.append(("companion, entities, time of day and events are present",
                       recs[0]["state"]["companion"]["name"] == COMP and recs[0]["state"]["day_time"] is not None
                       and len(recs[6]["state"]["events"]) > 0, recs[0]["state"]["companion"]))
        us = [int(l.split("us=")[1]) for l in log_lines() if l.startswith("DECISION ")]
        bad = [(n, d) for n, ok, d in checks if not ok]
        print(json.dumps(dict(records=len(recs), checks=len(checks), failed=bad, cost_us=dict(
            n=len(us), median=statistics.median(us), max=max(us)), bytes_per_record=LOG.stat().st_size // len(recs))))
        print(json.dumps(dict(sample=recs[2])))
    finally:
        stop()
        clear_site()


if __name__ == "__main__":
    main()
