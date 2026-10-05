#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
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
"""
How much of what a diff tool is asked about OmniDiff diffs by content: the change census's window
(`change_census.py`: every file changed by the most recent `--max-commits` non-merge commits of
each clone, through `edit_shape_stats.numstat_rows`), each change judged by OmniDiff itself -
`content_census` reads both blobs and says `text`, a content family (`pictures`, `fonts`, ...) or
`binary` (see that binary's doc for the outcomes, and why they are OmniDiff's verdict, not git's).
With `--diff` (the Makefile's default) every content pair is diffed, so a family counts only the
pairs that diff.

Two files, because a handful of repositories hold most changes (the change census): by key
(extension, or name for a file without one) and outcome, with how many repositories; and by
repository and outcome. One repository's 50 commits can touch 159,244 files
(rpm-software-management-distribution-gpg-keys, every one text), so the raw share of changes
measures a few giants. The summary on stderr gives three shares of changes diffed as text or
content: raw; capped, each repository's counts weighted by min(1, cap / its changes) - exactly
what sampling `--cap` of its changes uniformly would give on average, without the randomness; and
the mean of each repository's own share, every repository counting once.

Usage (from research/):
    uv run ./analysis/content_census.py [--repositories DIR] [--max-commits N] [--jobs N]
"""

import argparse
import collections
import csv
import json
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed

from change_census import key_of
from edit_shape_stats import numstat_rows

# The outcomes that are a diff a reader sees as content, not "Binary files differ".
UNSEEN = ("binary", "missing")


def covered(outcome):
    """True if `outcome` is a change OmniDiff diffs as text or content."""
    return outcome not in UNSEEN and ":" not in outcome


def census_of(repo, max_commits, tool, diff):
    """One repository's outcomes: a Counter of (key, outcome)."""
    rows = [(commit, path) for commit, path, _, _ in numstat_rows(repo, max_commits, True)]
    if not rows:
        return os.path.basename(repo), collections.Counter()
    command = [tool, "--repo", repo] + (["--diff"] if diff else [])
    result = subprocess.run(
        command,
        input="".join(f"{commit}\t{path}\n" for commit, path in rows),
        capture_output=True,
        text=True,
        errors="replace",
        check=False,
    )
    if result.returncode != 0:
        print(f"note: content_census failed in {os.path.basename(repo)}", file=sys.stderr)
    tally = collections.Counter()
    for line in result.stdout.splitlines():
        path, _, outcome = line.rpartition("\t")
        tally[(key_of(path), outcome)] += 1
    return os.path.basename(repo), tally


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repositories", default="/var/tmp/research/small/repositories")
    parser.add_argument("--max-commits", type=int, default=50)
    parser.add_argument("--jobs", type=int, default=os.cpu_count())
    parser.add_argument("--no-diff", action="store_true", help="recognise content, do not diff it")
    parser.add_argument(
        "--cap", type=int, default=1000, help="changes per repository the capped share counts"
    )
    parser.add_argument(
        "--resume",
        default=None,
        help="the per-repository resume file (default: content_census.partial.jsonl beside the "
        "repositories directory); delete it to start over",
    )
    parser.add_argument(
        "--tool",
        default=os.path.join(
            os.path.dirname(__file__), "..", "..", "target", "release", "content_census"
        ),
    )
    args = parser.parse_args()
    research_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    out_dir = os.path.join(research_dir, "data", "corpus_stats")

    repos = sorted(
        os.path.join(args.repositories, name)
        for name in os.listdir(args.repositories)
        if os.path.isdir(os.path.join(args.repositories, name, ".git"))
    )
    # Each repository's tally is appended to a resume file as it lands, so a run that stops
    # (a reboot, a stopped unit) picks up where it was rather than starting over.
    resume = args.resume or os.path.join(
        os.path.dirname(os.path.abspath(args.repositories)), "content_census.partial.jsonl"
    )
    by_repository = {}
    if os.path.exists(resume):
        with open(resume) as f:
            for line in f:
                record = json.loads(line)
                by_repository[record["repository"]] = collections.Counter(
                    {(key, outcome): count for key, outcome, count in record["tally"]}
                )
    todo = [repo for repo in repos if os.path.basename(repo) not in by_repository]
    print(f"{len(by_repository)} repositories from {resume}, {len(todo)} to go", file=sys.stderr)
    # Threads, not processes: each repository's work is two child processes (git log and
    # content_census), and a process pool once hung for hours with every worker idle.
    with ThreadPoolExecutor(args.jobs) as pool, open(resume, "a") as out:
        futures = [
            pool.submit(census_of, repo, args.max_commits, args.tool, not args.no_diff)
            for repo in todo
        ]
        for done, future in enumerate(as_completed(futures), 1):
            name, tally = future.result()
            by_repository[name] = tally
            record = {"repository": name, "tally": [[k, o, c] for (k, o), c in tally.items()]}
            out.write(json.dumps(record) + "\n")
            out.flush()
            if done % 250 == 0:
                print(f"  {done} of {len(todo)} repositories", file=sys.stderr, flush=True)

    by_key = collections.Counter()
    key_repositories = collections.Counter()
    outcomes_by_repository = {}
    for name, tally in by_repository.items():
        by_key.update(tally)
        key_repositories.update(tally.keys())
        outcomes = collections.Counter()
        for (_, outcome), count in tally.items():
            outcomes[outcome] += count
        outcomes_by_repository[name] = outcomes

    with open(os.path.join(out_dir, "content_census.csv"), "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["key", "outcome", "changes", "repositories"])
        for (key, outcome), count in sorted(by_key.items(), key=lambda item: (-item[1], item[0])):
            writer.writerow([key, outcome, count, key_repositories[(key, outcome)]])
    with open(os.path.join(out_dir, "content_census_repositories.csv"), "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["repository", "outcome", "changes"])
        for name in sorted(outcomes_by_repository):
            for outcome, count in sorted(outcomes_by_repository[name].items()):
                writer.writerow([name, outcome, count])

    print(summary(outcomes_by_repository, args.cap), file=sys.stderr)


def summary(by_repository, cap):
    """The outcomes' shares, and the three shares of changes diffed as text or content."""

    def counted(tally):
        return {outcome: count for outcome, count in tally.items() if outcome != "missing"}

    outcomes = collections.Counter()
    capped = collections.Counter()
    shares = []
    for tally in by_repository.values():
        tally = counted(tally)
        changes = sum(tally.values())
        if not changes:
            continue
        outcomes.update(tally)
        weight = min(1.0, cap / changes)
        for outcome, count in tally.items():
            capped[outcome] += weight * count
        shares.append(sum(c for o, c in tally.items() if covered(o)) / changes)

    def share(counter):
        total = sum(counter.values())
        return 100 * sum(c for o, c in counter.items() if covered(o)) / max(total, 1)

    total = sum(outcomes.values())
    lines = [f"{len(shares)} repositories with changes, {total} changed files"]
    for outcome, count in outcomes.most_common():
        lines.append(
            f"  {outcome:24} {count:9} {100 * count / max(total, 1):7.3f}%"
            f"  capped {100 * capped[outcome] / max(sum(capped.values()), 1):7.3f}%"
        )
    lines.append(
        f"diffed as text or content: {share(outcomes):.3f}% of changes; "
        f"{share(capped):.3f}% at most {cap} per repository; "
        f"{100 * sum(shares) / max(len(shares), 1):.3f}% per repository on average; "
        f"{sum(1 for s in shares if s == 1.0)} of {len(shares)} repositories entirely"
    )
    return "\n".join(lines)


if __name__ == "__main__":
    main()
