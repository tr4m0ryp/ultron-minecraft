"""Care, combat and pre-emption goals live: sleep_in_bed at night, eat_now, attack, guard,
and a creeper summoned during a mine goal (a reflex takes the body, the goal resumes).
Usage: care_combat.py [case ...]"""
import json, re, sys
from kit import *


def entity_count(kind, r=40):
    out = rc(f"execute if entity @e[type={kind},x={CX},y={Y},z={CZ},distance=..{r}]")
    m = re.search(r"Count: (\d+)", out)
    return int(m.group(1)) if m else 0


def nbt(path, name="Ultron"):
    out = rc(f"data get entity {name} {path}")
    m = re.search(r"data: (.*)$", out)
    return m.group(1) if m else out


def sleep():
    prep(night=True, comp_at=(CX - 3.5, Y + 1, CZ - 3.5))
    bx, bz = CX + 6, CZ + 4
    rc(f"setblock {bx} {Y + 1} {bz} red_bed[facing=north,part=foot]",
       f"setblock {bx} {Y + 1} {bz - 1} red_bed[facing=north,part=head]")
    time.sleep(1.5)
    n = task("sleep_in_bed")
    kind, text, _ = outcome(n, 40, "sleep_in_bed")
    sleeping = nbt("sleeping_pos")
    return dict(case="sleep_in_bed", outcome=kind, text=text, server_sleeping_pos=sleeping)


def eat():
    prep(items=[("hotbar.2", "bread", 4)], comp_at=(CX - 3.5, Y + 1, CZ - 3.5))
    rc("effect give Ultron hunger 6 255")
    time.sleep(6)
    rc("effect clear Ultron hunger")
    before = nbt("foodLevel")
    n = task("eat_now")
    kind, text, _ = outcome(n, 30, "eat_now")
    time.sleep(1)
    return dict(case="eat_now", outcome=kind, text=text, food_before=before,
                food_after=nbt("foodLevel"), bread_left=count_item("Ultron", "bread"))


def attack():
    prep(items=[("hotbar.0", "diamond_sword", 1)], comp_at=(CX - 3.5, Y + 1, CZ - 3.5))
    rc(f"summon zombie {CX + 8.5} {Y + 1} {CZ + 6.5} {{PersistenceRequired:1b}}")
    time.sleep(1)
    n = task("attack zombie")
    kind, text, _ = outcome(n, 40, "attack")
    return dict(case="attack", outcome=kind, text=text, zombies_left=entity_count("zombie"),
                bot_hp=nbt("Health"))


def guard():
    prep(items=[("hotbar.0", "diamond_sword", 1)], comp_at=(CX + 6.5, Y + 1, CZ + 6.5))
    rc(f"gamemode survival {COMP}", f"effect give {COMP} resistance 60 4")
    n = task("guard companion")
    time.sleep(2)
    rc(f"summon zombie {CX + 12.5} {Y + 1} {CZ + 10.5} {{PersistenceRequired:1b}}")
    t_end = time.time() + 40
    while time.time() < t_end and entity_count("zombie"):
        time.sleep(1)
    left = entity_count("zombie")
    wakes = [w[1:] for w in outcome(n, 0)[2]]
    rc(f"gamemode creative {COMP}")
    return dict(case="guard", zombies_left=left, companion_hp=nbt("Health", COMP),
                wakes=wakes[-3:])


def preempt():
    """mine stone 6 from a stone wall; a creeper is summoned next to the bot mid-goal."""
    prep(items=[("hotbar.0", "stone_pickaxe", 1)], comp_at=(CX - 3.5, Y + 1, CZ - 3.5))
    rc("gamerule mob_griefing false",
       f"fill {CX + 2} {Y + 1} {CZ - 2} {CX + 2} {Y + 2} {CZ + 2} stone")
    t0 = now_ms()
    n = task("mine stone 6")
    time.sleep(3)
    p = pos()
    rc(f"summon creeper {p[0] + 2} {p[1]} {p[2] + 1}")
    kind, text, wakes = outcome(n, 90, "mine")
    rs = rows(since_ms=t0)
    owners = []
    for r in rs:
        if not owners or owners[-1][0] != r["legs"]:
            owners.append((r["legs"], r["tick"]))
    goal_events = [w[1] for w in wakes]
    rc(f"kill @e[type=creeper,x={CX},y={Y},z={CZ},distance=..60]")
    return dict(case="preempt", outcome=kind, text=text,
                cobblestone=count_item("Ultron", "cobblestone"),
                legs_owner_sequence=[o[0] for o in owners][:16], goal_events=goal_events,
                bot_hp=nbt("Health"))


CASES = dict(sleep=sleep, eat=eat, attack=attack, guard=guard, preempt=preempt)

if __name__ == "__main__":
    names = sys.argv[1:] or list(CASES)
    build_site()
    start()
    try:
        for name in names:
            print(json.dumps(CASES[name]()), flush=True)
    finally:
        rc("time set noon")
        stop()
