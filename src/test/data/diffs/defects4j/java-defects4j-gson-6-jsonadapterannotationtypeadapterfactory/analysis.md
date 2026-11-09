# Accuracy analysis: java-defects4j-gson-6-jsonadapterannotationtypeadapterfactory

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 1060 nodes, 1060 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 5587: renderer 4, matcher 0 |
| Painting, Full | 0 bytes wrong of 5587: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The `typeAdapter = typeAdapter.nullSafe();` statement (line 67 before) is wrapped in a new `if
(typeAdapter != null) { ... }` (lines 67 and 69 after).

## Painting

Full matches exactly. Minimal, `Insert -> None`, line 67 after side, 4 bytes: the four single spaces
inside the inserted `if (typeAdapter != null) {`. The Minimal painting inserts the line as six token
spans (`if`, `(typeAdapter`, `!=`, `null)`, `{`, and the `}` on line 69) and leaves the spaces
between them unpainted; omnidiff's Minimal rendering paints the line as one Insert, inner spaces
included. Same with the human mapping.

`text_painting_findings.md` (rule 4) says whitespace inside a range is usually kept, which is what
omnidiff does; this painting chose otherwise. Both readings are defensible, and the bytes are only
separator spaces. `ambiguous`.

## Verdict

- Primary cause: `ambiguous`
- Other causes: none
- Where a fix would go: none: ambiguous (the painting splits an inserted line at its spaces, against
  the usual convention)
- Ground truth questionable: no
