"""Trivial baselines: the floor a decision model has to beat.

  python trivial.py cases.jsonl --stride 3
  python trivial.py cases_goals.jsonl
chance: expected accuracy of a uniform random pick. first: always the first listed option (yes for yes/no,
level 0 for scores). majority: the most frequent answer per question kind, read off the same cases (an
optimistic floor). rule: a keyword rule over the state text, goal cases only; it was written by the author of
those cases, so it is an upper bound for what a rule gets here, not an independent result.
"""
import argparse, collections, json, re


def options(c):
    q = c["question"]
    if q["type"] == "noul":
        return ["yes", "no"]
    if q["type"] == "score":
        return list(range(len(q["criteria"])))
    return list(q["criteria"])


RULES = [  # first match wins; reads the Chat field first, then the body state
    ("eat_now", lambda s, ch: int(re.search(r"hunger (\d+)", s).group(1)) <= 6),
    ("stay", lambda s, ch: re.search(r"\b(wait|stay|hold still|don't move)\b", ch)),
    ("guard", lambda s, ch: re.search(r"\b(guard|protect|cover me|watch my back|keep me safe)\b", ch)),
    ("sleep_in_bed", lambda s, ch: "bed " in s.split("nearby:")[1].split("|")[0] and re.search(r"night|dusk", s)),
    ("give", lambda s, ch: re.search(r"\b(give me|hand me|pass me|toss me|can i have)\b", ch)),
    ("craft", lambda s, ch: "crafting table" in s.split("nearby:")[1].split("|")[0]),
    ("collect", lambda s, ch: "dropped item" in s),
    ("place", lambda s, ch: re.search(r"\b(place|put|set|block the)\b", ch)),
    ("come_here", lambda s, ch: re.search(r"\b(come|over here)\b", ch)),
    ("go_to", lambda s, ch: re.search(r"\b(go to|head back|walk to|meet me|run to)\b", ch)),
    ("follow", lambda s, ch: re.search(r"\b(follow|stay with|keep up|stick with|tag along|let's go)\b", ch)),
    ("attack", lambda s, ch: re.search(r"kill|zombie|skeleton|spider|creeper", s)),
    ("mine", lambda s, ch: re.search(r" ore |stone|tree", s)),
]


def rule(c):
    s = c["state"].lower()
    ch = s.split("chat:")[1].split("|")[0] if "chat:" in s else ""
    for goal, test in RULES:
        if test(s, ch):
            return goal
    return "follow"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("cases"); ap.add_argument("--stride", type=int, default=1)
    a = ap.parse_args()
    cases = [json.loads(l) for l in open(a.cases)][::a.stride]
    n = len(cases)
    chance = sum(1 / len(options(c)) for c in cases)
    first = sum(options(c)[0] == c["answer"] for c in cases)
    by_kind = collections.defaultdict(collections.Counter)
    for c in cases:
        by_kind[c["kind"]][str(c["answer"])] += 1
    major = sum(cnt.most_common(1)[0][1] for cnt in by_kind.values())
    res = {"cases": a.cases, "n": n, "chance": round(chance, 2), "first_option": first, "majority_per_kind": major}
    if all(c["kind"] == "goal" for c in cases):
        res["rule"] = sum(rule(c) == c["answer"] for c in cases)
    print(json.dumps(res))


if __name__ == "__main__":
    main()
