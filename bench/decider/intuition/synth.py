"""Synthetic pairs in the real state format. Two kinds, both labelled synthetic and never in a test set.

  python synth.py      -> data/pairs_synth_vary.jsonl, data/pairs_synth_rules.jsonl

vary   every real training pair, 3 copies with the fields that do not carry the decision varied: coordinates
       shifted by one offset per copy, ticks and goal ids shifted, distances scaled, stack counts jittered,
       food 17..20 and hp 18..20. The label is the recorded one.
rules  states for decisions the recorded play does not contain (eating, sleeping, fighting, following, guarding,
       retreating, dusk), built as a grid over the fields and labelled by the written playing rules, not by a
       recorded decision. Their labels share an author with cases_nochat.jsonl, which was written first.
"""
import itertools, json, os, random, re
from data import DATA, real

IDLE = ("no event: the body has no goal and is standing idle. Take the next step of your project now "
        "(at night too: craft, build or fix at the base)")


def vary(p, rng):
    dx, dz, dt, did = rng.randint(-60, 60), rng.randint(-60, 60), rng.randint(200, 30000), rng.randint(1, 80)
    scale = rng.uniform(0.7, 1.4)

    def shift(text):
        text = re.sub(r"\bt=(\d+)", lambda m: f"t={int(m.group(1)) + dt}", text)
        text = re.sub(r"#(\d+)", lambda m: f"#{int(m.group(1)) + did}", text)
        text = re.sub(r"(?<= )(-?\d+) (\d+) (-?\d+)(?=[ :;,)]|$)",
                      lambda m: f"{int(m.group(1)) + dx} {m.group(2)} {int(m.group(3)) + dz}", text)
        return text

    sn = shift(p["snapshot"])
    sn = re.sub(r"pos (-?[\d.]+),(\d+),(-?[\d.]+)",
                lambda m: f"pos {float(m.group(1)) + dx:.1f},{m.group(2)},{float(m.group(3)) + dz:.1f}", sn)
    sn = re.sub(r"(near: [^;]*)", lambda m: re.sub(r" ([\d.]+)(?=,|$| )", lambda d: f" {float(d.group(1)) * scale:.1f}",
                                                    m.group(1)), sn)
    sn = re.sub(r"(held \w+ x)(\d+)", lambda m: m.group(1) + str(max(2, int(m.group(2)) + rng.randint(-5, 5))), sn)
    if re.search(r"hp 20 food 20", sn):
        sn = sn.replace("hp 20 food 20", f"hp {rng.choice((18, 19, 20, 20))} food {rng.choice((17, 18, 19, 20, 20))}", 1)
    return {**p, "event": shift(p["event"]), "snapshot": sn, "synthetic": "vary"}


def snap(rng, hp, food, held, dn, mode, near="nothing", comp=None, goal="none", last=None):
    pos = f"{rng.randint(-90, 90)}.5,{rng.randint(55, 74)},{rng.randint(-90, 90)}.5"
    s = f"t={rng.randint(300, 23000)} pos {pos} hp {hp} food {food} held {held} {dn} mode {mode} ; near: {near} ; "
    s += f"companion Mate {comp}m hp {rng.choice((12, 16, 20, 20))}" if comp else "companion not visible"
    return s + f" ; goal {goal}" + (f" ; last goal {last}" if last else "")


def rules(rng):
    out, T = [], lambda: rng.randint(300, 23000)
    done = ["goal_done: #{i} mine: mined 8 stone, picked up 8 cobblestone", "goal_done: #{i} go_to: arrived at 4 70 9",
            "goal_done: #{i} craft: crafted 4 stick", "goal_done: #{i} place: placed oak_planks at 3 71 2"]
    work = [["mine 8 stone"], ["go_to pos"], ["craft 4 stick"], ["place oak_planks pos", "place oak_planks pos"]]
    held, modes, hostiles = ["pickaxe", "axe", "other", "sword", "food x3"], ["mine", "build", "explore"], \
        ["zombie", "skeleton", "spider", "creeper", "husk"]

    def add(label, event, sn, hist):
        m = re.search(r"mode (\w+)", sn).group(1)
        out.append({"session": "synthetic", "event": event, "snapshot": sn, "mode": m, "history": hist, "label": label,
                    "detail": label, "synthetic": "rules"})

    for food, (d, w), h, dn in itertools.product((1, 2, 3, 4, 5, 6), zip(done, work), held[:3], ("day", "night")):
        ev = rng.choice((f"t={T()} " + d.format(i=rng.randint(2, 90)), "no event (timeout)", IDLE))
        add("eat_now", ev, snap(rng, rng.choice((6, 10, 14, 20)), food, h, dn, rng.choice(modes)), w)
    for kind, dist, hp, comp in itertools.product(hostiles[:4], (2.5, 4.0, 6.5), (20, 15, 11), (3.0, 6.0, None)):
        near = f"{kind} {dist}" + (f", player {comp}" if comp else "")
        ev = (f"t={T()} companion_danger Mate: hostile near companion or companion hurt" if comp else
              rng.choice((f"t={T()} goal_done: #{rng.randint(2, 90)} attack: killed {rng.choice(hostiles)}",
                          f"t={T()} goal_failed: #{rng.randint(2, 90)} sleep_in_bed: monsters nearby")))
        add("attack", ev, snap(rng, hp, rng.randint(12, 20), "sword", "night", "fight", near, comp),
            rng.choice((["follow"], ["guard"], ["mine 8 stone", "attack " + kind], ["sleep_in_bed"])))
    for kind, n, hp, dn in itertools.product(hostiles, (1, 2, 3), (3, 5, 7), ("night", "day")):
        near = f"{kind} x{n} {rng.uniform(2, 7):.1f}" if n > 1 else f"{kind} {rng.uniform(2, 7):.1f}"
        ev = rng.choice((f"t={T()} first_sighting: first {kind} this session", "no event (timeout)",
                         f"t={T()} goal_interrupted: goal \"mine stone 8\" has lost the legs to melee for 200 ticks"))
        add("set_mode", ev, snap(rng, hp, rng.randint(8, 20), rng.choice(held[:4]), dn, rng.choice(("mine", "fight"))
                                 , near), rng.choice(work + [["attack " + kind]]))
    for item, m, h in itertools.product(("white_bed", "red_bed"), modes, held[:4]):
        add("sleep_in_bed", rng.choice((f"t={T()} goal_done: #{rng.randint(2, 90)} place: placed {item} at 6 71 5",
                                        f"t={T()} night: night has begun")),
            snap(rng, 20, rng.randint(14, 20), h, "night", m, last=f"#{rng.randint(2, 90)} place done: placed {item} at 6 71 5"),
            [f"craft 1 {item}", f"place {item} pos"])
    for dist, w, h, dn in itertools.product((26.0, 31.0, 38.0, 45.0), work, held[:4], ("day", "night")):
        add("follow", f"t={T()} companion_far Mate", snap(rng, 20, 20, h, dn, rng.choice(modes), f"player {dist}", dist), w)
    for goal, ev, dist, dn in itertools.product(("follow", "guard"), ("no event (timeout)", "companion_far"),
                                                (4.0, 9.0, 28.0), ("day", "night")):
        ev = ev if ev.startswith("no") else f"t={T()} companion_far Mate"
        for _ in range(2):
            add("wait", ev, snap(rng, 20, rng.randint(14, 20), rng.choice(held[:4]), dn, "follow", f"player {dist}", dist,
                                 goal=f"#{rng.randint(2, 90)} {goal} Mate p0.5 running, 0 queued"), [goal, "wait"])
    for kind, dist, comp in itertools.product(hostiles[:3], (14.0, 18.0, 22.0), (3.0, 5.0, 7.0)):
        ev = rng.choice((f"t={T()} first_sighting: first {kind} this session",
                         f"t={T()} goal_done: #{rng.randint(2, 90)} attack: killed {kind}"))
        near = (f"{kind} {dist}, " if "first" in ev else "item 1.5, ") + f"player {comp}"
        add("guard", ev, snap(rng, rng.choice((14, 20)), 18, "sword", "night", rng.choice(("follow", "fight")), near, comp),
            rng.choice((["follow"], ["guard", "attack " + kind])))
    for dist, comp in itertools.product((8.0, 10.0, 12.0, 14.0), (2.0, 3.0, 4.0, 5.0)):
        add("arm", f"t={T()} first_sighting: first creeper this session",
            snap(rng, 20, 20, "sword", rng.choice(("day", "night")), "follow", f"creeper {dist}, player {comp}", comp,
                 goal=f"#{rng.randint(2, 90)} follow Mate p0.5 running, 0 queued"), ["follow", "wait"])
    for (d, w), h in itertools.product(zip(done, work), held[:4]):
        for _ in range(2):
            add("go_to", f"t={rng.randint(12000, 12200)} night: dusk, night begins in 1000 ticks",
                snap(rng, 20, rng.randint(12, 20), h, "day", "explore"), ["go_to pos"] + w)
    for n, dist in itertools.product((1, 2, 3, 4), (1.0, 1.5, 2.5, 3.5)):
        near = f"item x{n} {dist}" if n > 1 else f"item {dist}"
        add("collect", f"t={T()} goal_done: #{rng.randint(2, 90)} attack: killed passive",
            snap(rng, 20, rng.randint(6, 20), "sword", "day", "fight", near + ", passive x3 9.0"), ["go_to pos", "attack passive"])
    return out


def main():
    rng = random.Random(20261115)
    train, _ = real()
    v = [vary(p, rng) for p in train for _ in range(3)]
    r = rules(rng)
    for name, ps in (("vary", v), ("rules", r)):
        with open(os.path.join(DATA, f"pairs_synth_{name}.jsonl"), "w") as f:
            f.writelines(json.dumps(p) + "\n" for p in ps)
        print(name, len(ps))


if __name__ == "__main__":
    main()
