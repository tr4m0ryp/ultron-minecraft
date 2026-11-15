"""Structured state features for the non-neural arm: no language model, just fields parsed from the pair."""
import re

HOSTILE = ("creeper", "zombie", "husk", "drowned", "skeleton", "stray", "bogged", "spider", "cave_spider", "enderman",
           "witch")
REASONS = {"unreachable": "Unreachable", "occupied": "occupied by", "missing": "missing ingredients",
           "rejected": "did not accept", "no_support": "nothing to place against", "out_of_reach": "out of reach",
           "no_recipe": "no known recipe", "none_near": r"no \w+ (drops )?within", "stalled": "Stalled",
           "tool": "missing tool", "standing": "standing in the spot", "gave_up": "gave up", "monsters": "monsters"}
ITEM = (r"(?:mined|crafted|collected|smelted|gave) \d+ (\w+)", r"placed (\w+)", r"ingredients for (\w+)",
        r"recipe for (\w+)", r"killed (\w+)", r"no (\w+) (?:drops )?within", r"reach (\w+) drops", r"tool for (\w+)")


def event_parts(event):
    """-> (kind, goal name, item) of the last event in the batch; kinds: goal_done, timeout, idle, chat, ..."""
    last = event.split(" ; ")[-1]
    if last.startswith("no event"):
        return ("idle" if "idle" in last else "timeout"), "", ""
    if last == "session start":
        return "start", "", ""
    m = re.match(r"(?:t=\d+ )?(\w+)(?::? (?:#\d+ )?(\w+))?", last)
    kind, goal = m.group(1), (m.group(2) or "") if m.group(1).startswith("goal_") else ""
    item = next((x.group(1) for x in (re.search(r, last) for r in ITEM) if x), "")
    return kind, goal, item


def features(p):
    f, ev, sn = {}, p["event"], p["snapshot"]
    kind, goal, item = event_parts(ev)
    f["ev=" + kind], f["evgoal=" + goal], f["evitem=" + item] = 1, 1, 1
    f["evpair=" + kind + ":" + goal] = 1
    f["n_events"] = len(ev.split(" ; "))
    for name, pat in REASONS.items():
        if re.search(pat, ev):
            f["why=" + name] = 1
    hist = p["history"][-3:]
    for i, h in enumerate(reversed(hist), 1):
        w = [x for x in h.split() if not x.isdigit() and x != "pos"]
        f[f"h{i}=" + w[0]] = 1
        if len(w) > 1:
            f[f"h{i}item=" + w[-1]] = 1
    if not hist:
        f["h1=none"] = 1
    if len(hist) > 1 and hist[-1].split()[0] == hist[-2].split()[0]:
        f["h_repeat"] = 1
    f["mode=" + (p.get("mode") or "none")] = 1
    for key, pat in (("hp", r"\bhp (\d+)"), ("food", r"food (\d+)")):
        m = re.search(pat, sn)
        f[key] = int(m.group(1)) if m else 20
    f["food_low"], f["hp_low"] = int(f["food"] <= 6), int(f["hp"] <= 8)
    m = re.search(r"held (\w+)", sn)
    f["held=" + (m.group(1) if m else "none")] = 1
    f["night"] = int(" night " in sn)
    near = re.search(r"near: ([^;]*)", sn)
    for g in (near.group(1).split(", ") if near else []):
        m = re.match(r"(\w+)(?: x(\d+))? ([\d.]+)", g.strip())
        if m:
            k = "hostile" if m.group(1) in HOSTILE else m.group(1)
            f["near_" + k] = f.get("near_" + k, 0) + int(m.group(2) or 1)
            f["dist_" + k] = min(f.get("dist_" + k, 99.0), float(m.group(3)))
    m = re.search(r"companion \S+ ([\d.]+)m(?: hp (\d+))?", sn)
    f["companion"] = int(bool(m))
    if m:
        f["companion_dist"], f["companion_hp"] = float(m.group(1)), int(m.group(2) or 20)
    m = re.search(r"; goal (?:#\d+ )?(\w+)", sn)
    f["running=" + (m.group(1) if m else "none")] = 1
    m = re.search(r"last goal (?:#\d+ )?(\w+) (\w+)", sn)
    if m:
        f["last=" + m.group(1) + ":" + m.group(2)] = 1
    return f
