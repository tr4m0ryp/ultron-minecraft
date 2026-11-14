"""Stages a scenario into a running fake-world relay by appending control lines on time.

usage: python3 drive.py <run-dir> <scenario-file>
Scenario lines: `s<N>+<seconds> <control line>`, timed from the start of session N
(see fake::stage for control lines). Blank lines and lines starting with # are skipped.
"""
import os
import sys
import time


def parse(path):
    steps = []
    for raw in open(path):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        when, ctl = line.split(" ", 1)
        session, offset = when[1:].split("+")
        steps.append((int(session), float(offset), ctl))
    return sorted(steps)


def session_start(run, n):
    path = os.path.join(run, f"session-{n}.start")
    while not os.path.exists(path):
        if os.path.exists(os.path.join(run, "..", "STOP")):
            sys.exit(0)
        time.sleep(0.5)
    time.sleep(0.05)
    return int(open(path).read().strip()) / 1000


def main():
    run, scenario = sys.argv[1], sys.argv[2]
    ctl = os.path.join(run, "stage.ctl")
    for session, offset, line in parse(scenario):
        due = session_start(run, session) + offset
        time.sleep(max(0.0, due - time.time()))
        with open(ctl, "a") as f:
            f.write(line + "\n")
        print(f"{time.strftime('%H:%M:%S')} s{session}+{offset:g} {line}", flush=True)


if __name__ == "__main__":
    main()
