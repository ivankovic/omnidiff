# Accuracy analysis: csharp-lidarr-lidarr-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 3945 nodes, 3945 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 14146: renderer 2, matcher 0 |
| Painting, Full | 2 bytes wrong of 14146: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Two test assertions change their expected album: line 196 `_albums.Take(1)` becomes
`_albums.Skip(1).Take(1)` and line 205 `Skip(2)` becomes `Skip(3)`.

## Painting

Both presets are scored against the single `Only one solution` painting; the same 2 bytes disagree
in both, and they survive with the human mapping, so they are renderer-owned:

- Move -> None, line 196, the `.` before `Take` on both sides (before column 50, after column 58).
  The tree mapping (omnidiff's and the ground truth's agree) pairs the before `.` of
  `_albums.Take` with the after `.` of `….Take`, which sits 8 columns further right behind the
  inserted `Skip(1)`. omnidiff paints that column shift as a Move. The painting reads the edit
  textually: `.Skip(1)` (after columns 50-58) is the Insert and `.Take(1)` is untouched. A
  displaced single token inside one line, whose shift is entirely the inserted text in front of
  it, is painted Move. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Move painting in src/diff/text.rs for a token on the same row whose column
  shift is only inserted text in front of it
- Ground truth questionable: no
