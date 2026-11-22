"""Times each bench/*.elo program with a built `elomar` (best of N runs).

    python -I bench/run.py [path/to/elomar] [runs]

Build first with `cargo build --release -p elomar-cli`.
"""
import pathlib
import subprocess
import sys
import time

here = pathlib.Path(__file__).resolve().parent
exe = sys.argv[1] if len(sys.argv) > 1 else str(here.parent / "target" / "release" / "elomar")
runs = int(sys.argv[2]) if len(sys.argv) > 2 else 3

total = 0.0
for program in sorted(here.glob("*.elo")):
    best = None
    for _ in range(runs):
        start = time.perf_counter()
        done = subprocess.run([exe, "run", str(program)], capture_output=True, text=True)
        elapsed = time.perf_counter() - start
        if done.returncode != 0:
            sys.exit(f"{program.name} failed:\n{done.stderr}")
        best = elapsed if best is None else min(best, elapsed)
    total += best
    print(f"{program.stem:10} {best * 1000:8.0f} ms   {done.stdout.strip()[:60]}")
print(f"{'total':10} {total * 1000:8.0f} ms")
