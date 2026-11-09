# Accuracy analysis: java-defects4j-closure-136-methodcompilerpass

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (0 visible) of 3365 nodes, 3365 graded, N:M floor 0 visible |
| Painting, Minimal | 65 bytes wrong of 17872: renderer 65, matcher 0 |
| Painting, Full | 23 bytes wrong of 17872: renderer 23, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-136: `MethodCompilerPass.addPossibleSignature` no longer resolves named functions: the
`signatureAdded` flag, the whole `else if (node.getType() == Token.NAME) { ... }` branch (before
lines 107-123) and the trailing `if (!signatureAdded) { nonMethodProperties.add(name); }` are
replaced by a plain `else { nonMethodProperties.add(name); }` (after lines 104-106).

## Mapping

- 1 invisible mismatch, no visible ones: the outer `if_statement` node (before 103). The ground
  truth matches it (MatchButNotIdentical); omnidiff leaves the node itself unmapped (op Delete,
  `APTED("qualified_name")`) while its children map as the ground truth says.
  `grading-artifact`.

## Painting

Every disagreement survives with the human mapping, so all of it is the renderer's (matcher 0
bytes).

- Minimal, None -> Move, 64 bytes: `nonMethodProperties.add(name);` and its `{` / `}` move from
  the deleted `if (!signatureAdded)` block (before lines 125-127) into the new `else` block
  (after 105-107). The Minimal painting paints them Move; omnidiff's Minimal rendering leaves
  them unpainted (its Full rendering does paint the statement Move, and agrees with the Full
  painting there). `renderer`.
- Minimal, Delete -> None, 1 byte: the space between `else` and the deleted `if (...)` on before
  line 107, painted Delete by omnidiff and left alone by the painting. `renderer`.
- Full, None -> Move, 23 bytes: the Full painting has explicit Matches of `    } else` (before
  107 -> after 105, identical, rows shifted by the deleted line) and one space; identical sides
  grade as Move, while the `} else` was kept, not moved, and omnidiff leaves it unpainted.
  `grading-artifact`.

## Verdict

- Primary cause: `renderer`
- Other causes: `grading-artifact`
- Where a fix would go: Minimal move painting in `src/diff/text.rs`: an identical statement whose
  enclosing block was replaced should be painted Move as Full does.
- Ground truth questionable: no.
