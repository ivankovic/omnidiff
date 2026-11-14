#!/usr/bin/env python3
#
#  This file is part of the OmniDiff code diffing tool.
#
#  Copyright (C) 2026 Marko Ivankovic
#
#  This program is free software: you can redistribute it and/or modify
#  it under the terms of the GNU Affero General Public License as published
#  by the Free Software Foundation, version 3 of the License.
#
#  This program is distributed in the hope that it will be useful,
#  but WITHOUT ANY WARRANTY; without even the implied warranty of
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
#  GNU Affero General Public License for more details.
#
#  You should have received a copy of the GNU Affero General Public License
#  along with this program. If not, see <https://www.gnu.org/licenses/>.

"""Runs packaging/install.sh in containers of several distributions, end to end.

What `curl -fsSL https://ivankovic.github.io/omnidiff/install.sh | sh` does on a user's machine:
the script arrives on stdin, picks a way to install, and installs the latest release from GitHub
(and, on Debian and Ubuntu, from the project's apt repository). So this needs podman and the
network, installs the *released* omnidiff rather than this checkout's, and is not part of
`make test`.

Each scenario is an image, what the script's stdin and terminal are, its arguments, and what must
be true afterwards. The ones that matter most are about the apt repository's signing key, which
the script may only add when somebody said yes:

* with no terminal, on Debian, it must install the tarball and leave /etc/apt alone;
* with a terminal and the answer "n", the same;
* with a terminal and the answer "y", or with --yes, it must install through apt.

The terminal scenarios run podman under a pty this script owns and type the answer when the
question appears, since the question is put to /dev/tty, not to stdin.

Usage:  python3 scripts/install_e2e.py [--scenario NAME]... [--list]
"""

from __future__ import annotations

import argparse
import os
import pathlib
import pty
import select
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass

ROOT = pathlib.Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "packaging" / "install.sh"
QUESTION = b"signing key? [y/N]"
TIMEOUT_S = 600

# What a machine whose user just ran `curl ... | sh` has: curl and certificates. The base images
# of Debian and Ubuntu have neither; Fedora's has no tar.
APT_SETUP = "apt-get update -qq && apt-get install -y -qq curl ca-certificates >/dev/null"
DNF_SETUP = "dnf -y -q install tar gzip >/dev/null"

TARBALL_CHECKS = """
set -e
test -x "$HOME/.local/bin/omnidiff"
"$HOME/.local/bin/omnidiff" --version
test -s "$HOME/.local/share/man/man1/omnidiff.1"
test -s "$HOME/.local/share/bash-completion/completions/omnidiff"
test -s "$HOME/.local/share/zsh/site-functions/_omnidiff"
test -s "$HOME/.local/share/fish/vendor_completions.d/omnidiff.fish"
test -s "$HOME/.local/share/doc/omnidiff/LICENSE"
printf 'a\\nb\\n' > /tmp/a.txt; printf 'a\\nc\\n' > /tmp/b.txt
"$HOME/.local/bin/omnidiff" --headless --color never /tmp/a.txt /tmp/b.txt | grep -q c
"""
NO_APT_KEY = """
test ! -e /etc/apt/keyrings/omnidiff-archive-keyring.gpg
test ! -e /etc/apt/sources.list.d/omnidiff.list
"""
APT_CHECKS = """
set -e
test -s /etc/apt/keyrings/omnidiff-archive-keyring.gpg
grep -q 'signed-by=/etc/apt/keyrings/omnidiff-archive-keyring.gpg' /etc/apt/sources.list.d/omnidiff.list
dpkg -s omnidiff | grep -q '^Status: install ok installed'
omnidiff --version
test ! -e "$HOME/.local/bin/omnidiff"
"""


@dataclass
class Scenario:
    name: str
    image: str
    setup: str
    args: str
    checks: str
    # What to type when the script asks about the signing key; None runs it without a terminal.
    answer: str | None = None
    # Text the script's output must contain.
    expect: str = ""


SCENARIOS = [
    Scenario(
        "debian-no-terminal",
        "docker.io/library/debian:12",
        APT_SETUP,
        "",
        TARBALL_CHECKS + NO_APT_KEY,
        expect="nobody to ask",
    ),
    Scenario(
        "debian-answer-no",
        "docker.io/library/debian:12",
        APT_SETUP,
        "",
        TARBALL_CHECKS + NO_APT_KEY,
        answer="n",
    ),
    Scenario(
        "debian-answer-yes", "docker.io/library/debian:12", APT_SETUP, "", APT_CHECKS, answer="y"
    ),
    Scenario("ubuntu-yes-flag", "docker.io/library/ubuntu:24.04", APT_SETUP, "--yes", APT_CHECKS),
    Scenario(
        "ubuntu-method-tarball",
        "docker.io/library/ubuntu:24.04",
        APT_SETUP,
        "--method tarball",
        TARBALL_CHECKS + NO_APT_KEY,
    ),
    # busybox: its sh, its wget (there is no curl), its tar.
    Scenario("alpine", "docker.io/library/alpine:3", "true", "", TARBALL_CHECKS),
    Scenario("fedora", "registry.fedoraproject.org/fedora:43", DNF_SETUP, "", TARBALL_CHECKS),
    Scenario("arch", "docker.io/library/archlinux:latest", "true", "", TARBALL_CHECKS),
    Scenario(
        "prefix-and-version",
        "docker.io/library/alpine:3",
        "true",
        "--prefix /opt/omnidiff --version v0.5.0",
        "test -x /opt/omnidiff/bin/omnidiff && /opt/omnidiff/bin/omnidiff --version | grep -q 0.5.0"
        " && test -s /opt/omnidiff/share/man/man1/omnidiff.1",
    ),
]


def inner(scenario: Scenario) -> str:
    """What runs in the container: the setup, the script from stdin as `curl | sh` delivers it,
    then the checks."""
    return (
        f"set -e\n{scenario.setup}\n"
        f"sh -s -- {scenario.args} < /install.sh\n"
        f"echo '--- checks'\n{scenario.checks}\necho '--- ok'\n"
    )


def run_plain(command: list[str]) -> tuple[int, str]:
    done = subprocess.run(
        command,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=TIMEOUT_S,
        check=False,
    )
    return done.returncode, done.stdout.decode(errors="replace")


def run_with_terminal(command: list[str], answer: str) -> tuple[int, str]:
    """`command` under a pty; `answer` is typed once, when the question appears."""
    pid, master = pty.fork()
    if pid == 0:
        os.execvp(command[0], command)
    output = b""
    answered = False
    deadline = time.monotonic() + TIMEOUT_S
    while time.monotonic() < deadline:
        ready, _, _ = select.select([master], [], [], 1.0)
        if not ready:
            continue
        try:
            chunk = os.read(master, 65536)
        except OSError:
            break
        if not chunk:
            break
        output += chunk
        if not answered and QUESTION in output:
            os.write(master, (answer + "\n").encode())
            answered = True
    _, status = os.waitpid(pid, 0)
    text = output.decode(errors="replace")
    if not answered:
        return 1, text + "\ninstall_e2e: the script never asked about the signing key\n"
    return os.waitstatus_to_exitcode(status), text


def run_scenario(scenario: Scenario) -> tuple[bool, str]:
    command = ["podman", "run", "--rm", "-v", f"{SCRIPT}:/install.sh:ro"]
    if scenario.answer is not None:
        command += ["-it"]
    command += [scenario.image, "sh", "-c", inner(scenario)]
    if scenario.answer is None:
        code, output = run_plain(command)
    else:
        code, output = run_with_terminal(command, scenario.answer)
    ok = code == 0 and "--- ok" in output and scenario.expect in output
    return ok, output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--scenario", action="append", help="run only this scenario (repeatable)")
    parser.add_argument("--list", action="store_true", help="list the scenarios and exit")
    args = parser.parse_args()
    if args.list:
        for scenario in SCENARIOS:
            print(scenario.name)
        return 0
    if shutil.which("podman") is None:
        sys.exit("install_e2e: needs podman")
    known = {scenario.name for scenario in SCENARIOS}
    for name in args.scenario or []:
        if name not in known:
            sys.exit(f"install_e2e: no scenario named {name}; --list names them")
    failed = []
    for scenario in SCENARIOS:
        if args.scenario and scenario.name not in args.scenario:
            continue
        ok, output = run_scenario(scenario)
        print(f"{'ok  ' if ok else 'FAIL'} {scenario.name}", flush=True)
        if not ok:
            failed.append(scenario.name)
            print("\n".join("    " + line for line in output.splitlines()[-40:]))
    if failed:
        print(f"install_e2e: {len(failed)} failed: {', '.join(failed)}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
