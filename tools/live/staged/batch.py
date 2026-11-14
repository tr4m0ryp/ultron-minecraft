"""Run staged survival jobs in parallel, one bot and one walled arena per slot.

Usage: python3 batch.py <outdir> <slots> <job> [<job> ...]
  job = label|binary|only|seed|shield|waves|pillars   (empty fields: survival.py defaults)
  e.g. "full_s1|/path/motor-azalea||1|0|2,4"  "melee_s1||D2|1||"

Slot k runs bot Ultron (k = 0) or Ultron<k> in the arena 100*k blocks south of the first, so
up to 5 slots fit under the server's 5-player cap. The server stays at its normal 20 Hz tick
rate: Azalea's client loop ticks at a fixed 20 Hz and ignores the ticking-state packet, so a
raised server rate would run mobs faster than the bot (measured: bot 20.0 ticks/s at server
rates 20, 40, 80 and 100). Parallel arenas are the valid way to buy throughput. The global
gamerules are set once here and restored when the batch ends; arena forceloads are removed.
Each job's stdout goes to <outdir>/<label>.txt.
"""
import os, subprocess, sys, time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from arena import rc  # noqa: E402

HERE = Path(__file__).resolve().parent


def job_env(slot, seed, shield, waves, pillars):
    env = dict(os.environ, SURV_SLOT=str(slot), SURV_SHARED="1", ULTRON_BRIDGE="0",
               BOT_NAME="Ultron" if slot == 0 else f"Ultron{slot}", BOT_WHO=f"bot{slot}")
    for key, val in (("SURV_SEED", seed), ("SURV_SHIELD", shield), ("SURV_WAVES", waves),
                     ("SURV_PILLARS", pillars)):
        if val:
            env[key] = val
        else:
            env.pop(key, None)
    return env


def launch(job, slot, outdir):
    label, binary, only, seed, shield, waves, pillars = (job.split("|") + [""] * 7)[:7]
    args = [sys.executable, str(HERE / "survival.py"), label, binary or "-"]
    if only:
        args.append(only)
    out = open(Path(outdir) / f"{label}.txt", "w")
    return subprocess.Popen(args, env=job_env(slot, seed, shield, waves, pillars), stdout=out,
                            stderr=subprocess.STDOUT, cwd=str(HERE))


def run(outdir, slots, jobs):
    Path(outdir).mkdir(parents=True, exist_ok=True)
    rc("tick rate 20", "gamerule spawn_mobs false", "gamerule advance_time false",
       "gamerule mob_griefing false", "difficulty normal")
    queue, running = list(jobs), {}
    try:
        while queue or running:
            for slot in range(slots):
                if slot not in running and queue:
                    running[slot] = launch(queue.pop(0), slot, outdir)
                    time.sleep(3)
            time.sleep(2)
            for slot, proc in list(running.items()):
                if proc.poll() is not None:
                    del running[slot]
    finally:
        for proc in running.values():
            proc.terminate()
        rc("gamerule spawn_mobs true", "gamerule advance_time true", "gamerule mob_griefing true")
        for slot in range(slots):
            z0 = 1100 + 100 * slot
            rc(f"forceload remove {1200 - 16} {z0 - 16} {1229 + 17} {z0 + 29 + 17}")


if __name__ == "__main__":
    run(sys.argv[1], int(sys.argv[2]), sys.argv[3:])
