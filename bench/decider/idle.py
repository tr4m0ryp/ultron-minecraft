"""Idle gate and contamination monitor for timed runs.

gate() waits until no compiler is running and the 1-minute load average is under the limit, then returns a
snapshot (load, busy cores over a 5 s sample, CPU share of the game server). Monitor samples during a run:
busy cores that are not this process tree, and whether a compiler showed up.
"""
import os, subprocess, threading, time
import psutil

COMPILERS = ("cargo", "rustc", "cc1plus", "cc1", "ld", "ninja", "cmake")


def compilers():
    out = []
    for p in psutil.process_iter(["name"]):
        if p.info["name"] in COMPILERS:
            out.append(p.info["name"])
    return out


def java_share(sample=2.0):
    """CPU share (percent of one core) of the game server java process, or None."""
    for p in psutil.process_iter(["name", "cmdline"]):
        if p.info["name"] == "java" and "server.jar" in " ".join(p.info["cmdline"] or []):
            p.cpu_percent(None); time.sleep(sample)
            return round(p.cpu_percent(None), 1)
    return None


def busy_cores(sample=5.0):
    return round(psutil.cpu_percent(sample) / 100 * psutil.cpu_count(), 2)


def gate(limit=0.5, wait_s=600, settle=5.0):
    """Wait for an idle machine. -> snapshot dict; snapshot['gate'] says whether the load limit was met."""
    t0 = time.time()
    while True:
        load1, comp = os.getloadavg()[0], compilers()
        if not comp and load1 < limit:
            ok = "met"; break
        if time.time() - t0 > wait_s:
            ok = "compiler running" if comp else "load limit not met"; break
        time.sleep(5)
    return {"gate": ok, "limit": limit, "load1": round(os.getloadavg()[0], 2), "busy_cores": busy_cores(settle),
            "java_cpu_pct": java_share(), "compilers": compilers(), "waited_s": round(time.time() - t0)}


class Monitor:
    """Background sampler: CPU cores used by everything except this process tree, and compiler sightings."""

    def __init__(self, pid=None, period=2.0):
        self.proc, self.period = psutil.Process(pid or os.getpid()), period
        self.foreign, self.comp, self._stop = [], set(), threading.Event()

    def _own(self):
        ps = [self.proc] + self.proc.children(recursive=True)
        t = 0.0
        for p in ps:
            try:
                c = p.cpu_times(); t += c.user + c.system
            except psutil.Error:
                pass
        return t

    def _run(self):
        ncpu = psutil.cpu_count()
        own0, t0 = self._own(), time.time()
        psutil.cpu_percent(None)
        while not self._stop.wait(self.period):
            total = psutil.cpu_percent(None) / 100 * ncpu
            own1, t1 = self._own(), time.time()
            self.foreign.append(max(0.0, total - (own1 - own0) / (t1 - t0)))
            own0, t0 = own1, t1
            self.comp.update(compilers())

    def __enter__(self):
        self.th = threading.Thread(target=self._run, daemon=True); self.th.start(); return self

    def __exit__(self, *a):
        self._stop.set(); self.th.join()

    def summary(self):
        f = sorted(self.foreign) or [0.0]
        return {"foreign_cores_mean": round(sum(f) / len(f), 2), "foreign_cores_max": round(f[-1], 2),
                "compilers_seen": sorted(self.comp), "samples": len(self.foreign)}


if __name__ == "__main__":
    import json, sys
    print(json.dumps(gate(wait_s=int(sys.argv[1]) if len(sys.argv) > 1 else 0)))
