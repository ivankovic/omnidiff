# Accuracy analysis: java-defects4j-jacksondatabind-94-subtypevalidator

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1450 nodes, 1450 graded, N:M floor 0 visible |
| Painting, Minimal | 608 bytes wrong of 10158: renderer 608, matcher 0 |
| Painting, Full | 829 bytes wrong of 10158: renderer 829, matcher 0 |
| Ground-truth invariant violations | 4 |

## The change

Adds a `PREFIX_C3P0` constant (line 23 after) and a new `} else if (full.startsWith(PREFIX_C3P0)) {`
branch (line 100 after) that blocks c3p0 `...DataSource` classes. The six `// [databind#1737]...` /
`// com.mchange...` comment lines, which before sat at the end of the Spring branch's `for` body
(lines 98-103), now open the new branch (lines 101-106 after); the `for` and Spring-branch closing
braces move above them.

## Painting

No mapping mismatches: the ground truth and omnidiff map every comment and brace to the same
counterpart. The disagreement is which of the reordered lines is shown as moved, the choice
`move_attribution.md` describes; all of it is renderer-owned (identical with the human mapping).

- Both presets, `None -> Move`, lines 98-103 before and 101-106 after: both paintings mark the six
  comment lines Move (Minimal 608 bytes; Full 800 bytes with their indentation). The comments
  changed parent, from the `for` body to the new `else if` block.
- Full only, `Move -> None`, lines 104-105 before and 99-100 after (29 bytes): omnidiff's Full
  rendering instead marks the two closing braces `}` / `            }` Move, the shorter group that
  crossed the comments; the paintings leave them alone. Minimal paints neither.

Both readings describe the same mapping. The paintings' reading follows the structure (the comments
are the nodes whose parent changed; the braces still close the same blocks), while omnidiff's
follows the walk order and so shows the comments as unmoved. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `ambiguous`
- Where a fix would go: src/diff/text.rs move attribution (reconcile_moves / crossed_backwards walk
  order): prefer nodes whose parent changed
- Ground truth questionable: no
