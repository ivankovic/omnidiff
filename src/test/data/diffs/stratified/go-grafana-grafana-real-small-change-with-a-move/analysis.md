# Accuracy analysis: go-grafana-grafana-real-small-change-with-a-move

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 511 nodes, 511 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 2021: renderer 0, matcher 0 |
| Painting, Full | 15 bytes wrong of 2021: renderer 15, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The `"k8s.io/apimachinery/pkg/runtime/schema"` import moves from before line 7 into its own
group at after line 4. Inside a struct literal, the comment `// Playlist tables are still being
used` (before line 25) is replaced by `SkipWhenMissing: true, // playlists may not exist at all`.
gofmt realigns `RenameTables:    []string{},`.

## Painting

Minimal matches exactly. All Full runs are the same with the human mapping, so they are the
renderer's.

- `None -> Move` / `None -> Update`, 13 bytes. The leading tab of the moved import (before line 7,
  after line 4). The `\t\t// ` of the old comment line (before line 25), and the `\t\t` and
  ` // ` of after line 26. The Full painting matches the comment as moved to the end of the new
  line, and includes its indentation and the `//` marker. Omnidiff's Full ranges cover only the
  import path and the comment text. `renderer`.
- `Move -> None`, 2 bytes: the trailing `,` of `RenameTables: []string{},` (before line 26, after
  line 27). gofmt's realignment shifted it, and omnidiff paints it alone as Move. The painting
  leaves the line unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering in `src/diff/text.rs`. Include indentation and the
  comment marker in a moved line's range, and do not paint a lone shifted trailing comma as Move.
- Ground truth questionable: no
