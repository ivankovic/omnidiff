# Accuracy analysis: rust-cost-optimization

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 418 nodes, 418 graded, N:M floor 0 visible |
| Painting, Minimal | 38 bytes wrong of 1456: renderer 38, matcher 0 |
| Painting, Full | 37 bytes wrong of 1456: renderer 37, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Handmade, after commit 81bf47c ("Best cost optimization, 10% faster"): adds an early `if cost >=
best_cost { continue; }` with a comment inside `solve`'s loop (after lines 8-11), and rewrites the
comment on line 4 from `// Original approach: explore all possibilities without early
termination` to `// Optimized approach: early termination when current cost exceeds best`.

## Painting

The mapping matches the ground truth everywhere; every disagreement is present with the human
mapping too.

- Line 4 comment, `Update -> None`, 37 bytes (both presets): both paintings paint the comment
  update only from column 26/27 (`explore all ...` / `early termination ...`), leaving `Original`
  -> `Optimized` unpainted. omnidiff narrows the update to one range from the first differing
  character (after the shared `O`) to the end, so it paints `riginal` / `ptimized` (15 bytes),
  which did change and which the painting omits, and the shared ` approach: ` between them (22
  bytes over both sides), which did not. The first part is the painting's (`ground-truth`), the
  second the renderer's single-range narrowing of a comment update (`renderer`).
- Minimal, 1 byte, `None -> Insert`: the space after `//` on the inserted comment `// Early
  termination optimization` (after line 8) is left unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`
- Where a fix would go: `src/diff/text.rs` `update_ranges` / `intra_node_update_ranges`
  (split a comment update into word-level ranges so a shared middle stays unpainted); the gap
  after `//`; the painting's omission of `Original` -> `Optimized` is Marko's call
- Ground truth questionable: yes: both paintings leave the changed word `Original` ->
  `Optimized` on line 4 unpainted while painting the rest of the comment as updated
