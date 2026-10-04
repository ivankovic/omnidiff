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
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
#  GNU Affero General Public License for more details.
#
#  You should have received a copy of the GNU Affero General Public License
#  along with this program. If not, see <https://www.gnu.org/licenses/>.

"""End-to-end test of the picture view's graphics detection inside tmux (`make test-graphics`).

The bug it guards: one tmux session, two clients - kitty on a desk, Termius on an e-ink tablet.
tmux passes the graphics query to every client, kitty answers that it draws pictures, and Termius
is sent kitty's placeholder characters, which it shows as rows of crossed-out boxes.

This starts a private tmux server (its own socket, no config) with one session and attaches two
clients to it, each on a pseudo-terminal this script plays the terminal for:

* **kitty**: names itself `kitty(0.44.0)` when tmux asks, and answers kitty's graphics queries
  `OK` (its `TERM` is `xterm-256color`, as over an ssh that does not carry `xterm-kitty`, so it is
  the name that tells it apart);
* **termius**: `TERM=xterm-256color`, answers no graphics query.

It then types `omnidiff util graphics` through one client at a time - typing makes it the client
tmux calls current - and checks what OmniDiff would draw with: half blocks for Termius although
kitty answers the query, kitty for kitty, and half blocks for kitty under
`OMNIDIFF_GRAPHICS=halfblocks`.

Usage: scripts/graphics_e2e.py path/to/omnidiff    (needs tmux and Python 3; no third-party code)
"""

import fcntl
import os
import pty
import re
import select
import shutil
import struct
import subprocess
import sys
import tempfile
import termios
import threading
import time

ROWS, COLUMNS = 30, 100


class Client:
    """One tmux client on a pseudo-terminal, with a thread answering what its terminal would."""

    def __init__(self, name, term, socket, env, graphics, version):
        self.name = name
        self.graphics = graphics
        self.version = version
        self.seen = b""
        pid, fd = pty.fork()
        if pid == 0:
            os.environ.clear()
            os.environ.update(env, TERM=term)
            os.execvp("tmux", ["tmux", "-L", socket, "-f", "/dev/null", "attach", "-t", "e2e"])
        self.pid, self.fd = pid, fd
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLUMNS, 0, 0))
        self.stop = False
        self.thread = threading.Thread(target=self.answer, daemon=True)
        self.thread.start()

    def answer(self):
        """Reads what tmux sends the terminal and answers its queries, as the terminal would."""
        pending = b""
        while not self.stop:
            ready, _, _ = select.select([self.fd], [], [], 0.1)
            if not ready:
                continue
            try:
                chunk = os.read(self.fd, 65536)
            except OSError:
                return
            self.seen += chunk
            pending += chunk
            replies = []
            # Only what tmux asks for and reads: an answer to anything else would arrive in the
            # pane as typed keys. The terminal's name and version (XTVERSION), which tmux keeps as client_termtype.
            if self.version:
                for _ in re.finditer(rb"\x1b\[>0?q", pending):
                    replies.append(b"\x1bP>|" + self.version + b"\x1b\\")
            # kitty's graphics protocol: every query is answered OK, under its own id.
            if self.graphics:
                for match in re.finditer(rb"\x1b_G([^\x1b;]*)", pending):
                    found = re.search(rb"i=(\d+)", match.group(1))
                    if found:
                        replies.append(b"\x1b_Gi=" + found.group(1) + b";OK\x1b\\")
            # Through tmux's passthrough, wrapped with the graphics query (tmux does not ask these
            # itself): the cell size in pixels, and the status report that ends the query.
            for _ in re.finditer(rb"\x1b\[16t", pending):
                replies.append(b"\x1b[6;20;10t")
            for _ in re.finditer(rb"\x1b\[5n", pending):
                replies.append(b"\x1b[0n")
            for reply in replies:
                os.write(self.fd, reply)
            # Keep a tail, for a query split across reads.
            pending = pending[-64:]

    def type(self, text):
        os.write(self.fd, text.encode())

    def close(self):
        self.stop = True
        try:
            os.kill(self.pid, 9)
        except ProcessLookupError:
            pass
        os.waitpid(self.pid, 0)


def tmux(socket, env, *args):
    return subprocess.run(
        ["tmux", "-L", socket, "-f", "/dev/null", *args],
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    omnidiff = os.path.abspath(sys.argv[1])
    if shutil.which("tmux") is None:
        sys.exit("graphics_e2e: needs tmux on PATH (make test-graphics runs it in podman if not)")
    socket = f"omnidiff-e2e-{os.getpid()}"
    work = tempfile.mkdtemp(prefix="omnidiff-graphics-")
    # No TMUX: this may itself run inside tmux, and the private server is not that one.
    env = {
        "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
        "HOME": work,
        "LANG": "C.UTF-8",
        "SHELL": "/bin/sh",
    }
    started = tmux(
        socket,
        {**env, "TERM": "xterm-256color"},
        "new-session",
        "-d",
        "-s",
        "e2e",
        "-x",
        str(COLUMNS),
        "-y",
        str(ROWS),
        "/bin/sh",
    )
    if started.returncode != 0:
        sys.exit(f"graphics_e2e: tmux new-session failed: {started.stderr}")
    tmux(socket, env, "set", "-g", "allow-passthrough", "on")
    kitty = Client("kitty", "xterm-256color", socket, env, graphics=True, version=b"kitty(0.44.0)")
    termius = Client("termius", "xterm-256color", socket, env, graphics=False, version=None)
    failures = []
    try:
        time.sleep(1.5)
        # A fresh prompt for the probes, past anything the attaching left on the line.
        termius.type("\x03\r")
        time.sleep(0.5)
        clients = tmux(socket, env, "list-clients", "-F", "#{client_termname} #{client_termtype}")
        print(f"clients:\n{clients.stdout.strip()}")

        def probe(client, name, prefix=""):
            # stdout stays the terminal: the query is written there, and a file would swallow it.
            # The answer is read back off the pane, between markers.
            client.type(
                f"clear; echo BEGIN-{name}; {prefix}{omnidiff} util graphics; echo END-{name}\r"
            )
            deadline = time.time() + 90
            while time.time() < deadline:
                screen = tmux(socket, env, "capture-pane", "-p", "-t", "e2e").stdout
                found = re.search(rf"^BEGIN-{name}\n(.*?)^END-{name}$", screen, re.M | re.S)
                if found:
                    answer = " ".join(found.group(1).split("\n")).strip()
                    print(f"{name}: {answer!r}")
                    return answer
                time.sleep(0.2)
            print(f"{name}: no answer; the pane shows:\n{screen}")
            return ""

        cases = [
            (termius, "termius", "", "halfblocks", "xterm-256color"),
            (kitty, "kitty", "", "kitty", "kitty(0.44.0)"),
            (kitty, "kitty-forced", "OMNIDIFF_GRAPHICS=halfblocks ", "halfblocks", "OMNIDIFF"),
            (termius, "termius-again", "", "halfblocks", "xterm-256color"),
        ]
        for client, name, prefix, protocol, why in cases:
            answer = probe(client, name, prefix)
            if answer.split()[:1] != [protocol] or why not in answer:
                failures.append(f"{name}: expected {protocol} ({why}), got {answer!r}")
        if b"\x1b_G" not in kitty.seen:
            failures.append(
                "the kitty client was never asked about graphics: the test proves nothing"
            )
    finally:
        kitty.close()
        termius.close()
        tmux(socket, env, "kill-server")
        shutil.rmtree(work, ignore_errors=True)
    if failures:
        print("FAILED:\n  " + "\n  ".join(failures))
        sys.exit(1)
    print("graphics_e2e: ok")


if __name__ == "__main__":
    main()
