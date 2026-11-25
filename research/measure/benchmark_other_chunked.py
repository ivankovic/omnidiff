#!/usr/bin/env python3
"""Run `benchmark_other` over the fixture corpus in chunks, resumably, and merge the CSVs.

A full tool-comparison run is hours of eight external tools on ~1,800 fixtures. Run as one
process it loses everything when anything goes wrong - an out-of-memory kill, a tool that hangs,
or (2026-11-25, on the laptop) the machine itself resetting under sustained all-core load. This
driver splits the fixture list `benchmark_other --list-fixtures` prints into chunks, runs one
`benchmark_other` process per chunk with `--fixtures`, and keeps each chunk's CSV in a work
directory. A chunk whose CSV exists is not run again, so re-running the same command after a crash
picks up at the first missing chunk. When every chunk is there, the CSVs are merged into the
output file in fixture order.

Why chunks and not a checkpoint inside the binary: a chunk is an ordinary `--fixtures` run, so it
needs no second code path for reading rows back, and a chunk process holds only its own fixtures'
temp files and parse trees. The warm GumTree and BDiff batches start one JVM or interpreter per
chunk rather than per run; their per-fixture numbers time only the tool's own call, so the chunk
size does not change them.

The work directory records the fixture list it was started with. If the corpus changed since
(fixtures added or removed), the driver refuses to mix the two states and asks for `--fresh`.

Rows are comparable only within one run (data/comparison/PROVENANCE.md): run all chunks of a timing
refresh on one machine, under one setup, without long gaps.
"""

import argparse
import csv
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
BINARY = REPO_ROOT / "target" / "release" / "benchmark_other"
DEFAULT_OUTPUT = {
    "accuracy": REPO_ROOT / "research/data/comparison/benchmark_accuracy.csv",
    "timing": REPO_ROOT / "research/data/comparison/benchmark_other.csv",
}
FLAG = {"accuracy": "--accuracy-csv", "timing": "--csv"}


def build():
    subprocess.run(
        [
            "cargo",
            "build",
            "--release",
            "--features",
            "test-fixtures",
            "--bin",
            "benchmark_other",
        ],
        cwd=REPO_ROOT,
        check=True,
    )


def list_fixtures():
    out = subprocess.run(
        [str(BINARY), "--list-fixtures"],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return [line for line in out.splitlines() if line]


def chunks(names, size):
    return [names[i : i + size] for i in range(0, len(names), size)]


def prepare_work_dir(work_dir, names, fresh):
    manifest = work_dir / "fixtures.txt"
    if fresh and work_dir.exists():
        shutil.rmtree(work_dir)
    work_dir.mkdir(parents=True, exist_ok=True)
    listed = "\n".join(names) + "\n"
    if manifest.exists():
        if manifest.read_text() != listed:
            sys.exit(
                f"{work_dir} was started on a different fixture list; re-run with --fresh to "
                "discard its chunks"
            )
    else:
        manifest.write_text(listed)


def run_chunk(index, total, names, mode, work_dir, extra, timeout):
    final = work_dir / f"chunk-{index:04d}.csv"
    if final.exists():
        return False
    partial = work_dir / f"chunk-{index:04d}.csv.partial"
    log = work_dir / f"chunk-{index:04d}.log"
    command = [str(BINARY), FLAG[mode], str(partial), "--fixtures", ",".join(names), *extra]
    started = time.monotonic()
    for attempt in (1, 2):
        with open(log, "w") as out:
            try:
                result = subprocess.run(
                    command,
                    cwd=REPO_ROOT,
                    stdout=out,
                    stderr=subprocess.STDOUT,
                    timeout=timeout,
                    check=False,
                )
                ok = result.returncode == 0 and partial.exists()
            except subprocess.TimeoutExpired:
                ok = False
        if ok:
            partial.rename(final)
            print(
                f"[{index + 1}/{total}] {len(names)} fixtures in {time.monotonic() - started:.0f}s",
                flush=True,
            )
            return True
        print(f"[{index + 1}/{total}] attempt {attempt} failed, see {log}", flush=True)
    sys.exit(f"chunk {index} failed twice; fix the cause and re-run to resume")


def merge(work_dir, count, output):
    header = None
    rows = []
    for index in range(count):
        with open(work_dir / f"chunk-{index:04d}.csv", newline="") as f:
            reader = csv.reader(f)
            chunk_header = next(reader)
            if header is None:
                header = chunk_header
            elif chunk_header != header:
                sys.exit(f"chunk {index} has a different header from chunk 0")
            rows.extend(reader)
    rows.sort(key=lambda row: row[header.index("solution")])
    output.parent.mkdir(parents=True, exist_ok=True)
    tmp = output.with_suffix(output.suffix + ".partial")
    with open(tmp, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(header)
        writer.writerows(rows)
    os.replace(tmp, output)
    print(f"Wrote {output} ({len(rows)} rows from {count} chunks)")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("mode", choices=["accuracy", "timing"])
    parser.add_argument("--output", type=Path, help="merged CSV (default: the committed one)")
    parser.add_argument("--chunk-size", type=int, default=25)
    parser.add_argument(
        "--work-dir",
        type=Path,
        help="where chunk CSVs and logs live (default: /var/tmp/omnidiff-benchmark-chunks/<mode>)",
    )
    parser.add_argument("--fresh", action="store_true", help="discard existing chunks first")
    parser.add_argument(
        "--limit", type=int, help="only the first N fixtures (a smoke test of the driver itself)"
    )
    parser.add_argument(
        "--pause", type=float, default=0, help="seconds to idle between chunks, to let it cool"
    )
    parser.add_argument(
        "--timeout", type=float, default=3 * 3600, help="seconds before a chunk counts as hung"
    )
    args, extra = parser.parse_known_args()

    output = args.output or DEFAULT_OUTPUT[args.mode]
    work_dir = args.work_dir or Path("/var/tmp/omnidiff-benchmark-chunks") / args.mode

    build()
    names = list_fixtures()
    if args.limit is not None:
        names = names[: args.limit]
    if not names:
        sys.exit("benchmark_other listed no fixtures")
    prepare_work_dir(work_dir, names, args.fresh)
    parts = chunks(names, args.chunk_size)
    print(f"{len(names)} fixtures in {len(parts)} chunks of up to {args.chunk_size}, in {work_dir}")
    for index, part in enumerate(parts):
        ran = run_chunk(index, len(parts), part, args.mode, work_dir, extra, args.timeout)
        if ran and args.pause:
            time.sleep(args.pause)
    merge(work_dir, len(parts), output)


if __name__ == "__main__":
    main()
