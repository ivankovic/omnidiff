# Accuracy analysis: rust-vercel-next-refactoring-multi-merge-with-update

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 62 (43 visible) of 918 nodes, 980 graded, N:M floor 43 visible |
| Painting, Minimal | 605 bytes wrong of 3176: renderer 537, matcher 317 |
| Painting, Full | 485 bytes wrong of 3176: renderer 279, matcher 369 |
| Ground-truth invariant violations | 36 |

## The change

`turbo-tasks-backend/src/database/by_key_space.rs`: the two key spaces `ForwardTaskCache` and
`ReverseTaskCache` are merged into one `TaskCache`. Each pair of lines (`forward_task_cache` /
`reverse_task_cache`: struct field, constructor, `get`, `get_mut`, `iter`) becomes one
`task_cache` line, five 2:1 merges.

## Mapping

- **All 62 mismatches (43 visible, 19 invisible) are the `reverse_*` lines** (before lines 8, 18,
  28, 38, 48). The ground truth maps both the `forward_*` and the `reverse_*` line onto the single
  after line (all-to-all groups); omnidiff maps the `forward_*` line and deletes the `reverse_*`
  one (`APTED("qualified_name")`). The N:M floor is 43 visible, equal to the visible count: no
  one-to-one mapping can do better. `nm-floor`.

## Painting

Only a `Minimal` painting exists; Full is scored against it too. Each of the five merges is one
2:1 `Match` entry spanning the whole of both before lines and the after line.

- **Minimal, Delete -> Update, 247 bytes, the five `reverse_*` lines.** omnidiff deletes them
  (the 1:1 mapping above); the painting updates them into the merged line. 68 of these bytes
  (the `reverse_` prefixes and `Reverse` words) differ from the human-mapping rendering; the other
  179 are the shared text discussed next. `nm-floor` (matcher-owned).
- **Minimal, None -> Update, 358 bytes, the `forward_*` lines and all five after lines.** The
  painting paints each whole line Update; omnidiff narrows the update to the differing words
  (`forward_`, `Forward`) and leaves `task_cache`, `TaskCache`, `KeySpace::`, `: T,`,
  `=> &self.` unpainted. The same with the human mapping. The ground-truth lint flags 36 of these
  spans (`invariants.txt` [16]: a Minimal painting paints `task_cache` / `TaskCache` against
  `forward_task_cache` / `ForwardTaskCache` where only `forward_` differs). `ground-truth`.
- **Full, Delete -> None, 299 bytes, the `reverse_*` lines.** As above, plus Full paints the
  indentation of the deleted lines, which the Minimal-style painting leaves out (the label at the
  first byte is the indentation's). `nm-floor`, with the indentation a `grading-artifact`.
- **Full, None -> Update, 118 bytes**: the same whole-line painting against shared text
  (`KeySpace::`, `=> &self.`, `: T,`). `ground-truth`.
- **Full, Move -> Update, 68 bytes**: punctuation of the updated lines (`: factory(KeySpace::`,
  `),`, `,`) that shifted row because a line above was removed is painted Move by Full; the
  painting has it inside the Update. Same with the human mapping. `renderer`.

## Verdict

- Primary cause: `nm-floor`
- Other causes: `ground-truth`, `renderer`, `grading-artifact`
- Where a fix would go: none for the mapping (2:1 merges); the whole-line Minimal Update spans are
  Marko's call; Full's Move on row-shifted punctuation is in `diff::text`
- Ground truth questionable: yes: the Minimal painting paints the shared `task_cache` /
  `TaskCache` words of every merged line (36 lint hits, [16])
