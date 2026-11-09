#!/usr/bin/env python3
#
#  This file is part of the OmniDiff code diffing tool.
#
#  Copyright (C) 2026 Marko Ivankovic
#
#  This program is free software: you can redistribute it and/or modify
#  it under the terms of the GNU Affero General Public License as published
#  by the Free Software Foundation, either version 3 of the License, or
#  (at your option) any later version.
#
#  This program is distributed in the hope that it will be useful,
#  but WITHOUT ANY WARRANTY; without even the implied warranty of
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#  GNU Affero General Public License for more details.
#
#  You should have received a copy of the GNU Affero General Public License
#  along with this program.  If not, see <https://www.gnu.org/licenses/>.
"""Prints the name of the machine this runs on, as `research/data/machines.csv` registers it.

Wall-clock figures belong to the hardware that measured them, so every timing baseline and speed
table is kept per machine, in a file named after it (`research/data/quality/timing/<machine>.csv`,
`research/data/performance/content_speed/<machine>.csv`). This is how the Makefile picks the file.

A machine is recognised by its CPU's model name, the one field that tells the project's machines
apart and does not change with a hostname or an OS reinstall. A CPU that is not registered is an
error: add a row for it rather than let its numbers land in another machine's files.

Usage:  python3 scripts/machine.py           the registered name of this machine
        python3 scripts/machine.py NAME      NAME, checked against the registry (MACHINE=NAME)
"""

import csv
import platform
import subprocess
import sys
from pathlib import Path

REGISTRY = Path(__file__).resolve().parent.parent / "research" / "data" / "machines.csv"


def cpu_model() -> str:
    """The CPU's model name, as the OS reports it; empty if it does not."""
    if platform.system() == "Darwin":
        result = subprocess.run(
            ["sysctl", "-n", "machdep.cpu.brand_string"],
            capture_output=True,
            text=True,
            check=False,
        )
        return " ".join(result.stdout.split())
    try:
        cpuinfo = Path("/proc/cpuinfo").read_text()
    except OSError:
        return " ".join(platform.processor().split())
    for line in cpuinfo.splitlines():
        key, _, value = line.partition(":")
        if key.strip() == "model name":
            return " ".join(value.split())
    return ""


def registry() -> dict[str, str]:
    """Machine name -> CPU model name."""
    with REGISTRY.open(newline="") as f:
        return {row["machine"]: " ".join(row["cpu"].split()) for row in csv.DictReader(f)}


def main(argv: list[str]) -> int:
    machines = registry()
    if len(argv) > 1:
        name = argv[1]
        if name not in machines:
            known = ", ".join(sorted(machines))
            print(f"error: no machine '{name}' in {REGISTRY} (known: {known})", file=sys.stderr)
            return 1
        print(name)
        return 0
    cpu = cpu_model()
    for name, registered in machines.items():
        if registered == cpu:
            print(name)
            return 0
    print(
        f"this machine's CPU ({cpu or 'unknown'}) is not registered in {REGISTRY}; add a row "
        f'`<name>,{cpu},"<cores, RAM>"`, or pass MACHINE=<name> to time as a registered one',
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
