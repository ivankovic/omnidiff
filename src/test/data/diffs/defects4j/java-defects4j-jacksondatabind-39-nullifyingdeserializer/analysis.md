# Accuracy analysis: java-defects4j-jacksondatabind-39-nullifyingdeserializer

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 634 nodes, 643 graded, N:M floor 6 visible |
| Painting, Minimal | 0 bytes wrong of 3534: renderer 0, matcher 0 |
| Painting, Full | 40 bytes wrong of 3534: renderer 40, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`deserialize()` now skips the fields of an object token by token: the lone `p.skipChildren();` (line
35 before) is wrapped in `if (p.hasToken(JsonToken.FIELD_NAME)) { while (true) { ... } } else { ...
}` (lines 35-45 after), and the call appears in both branches (lines 41 and 44 after).

## Mapping

No mapping mismatches. The ground truth maps the one before `p.skipChildren();` to both after copies
(1:2), which is why `graded` (643) exceeds `nodes` (634) and the N:M floor is 6 visible nodes;
omnidiff's one-to-one mapping still scores 0 mismatches against it.

## Painting

Minimal matches exactly. Full, 40 bytes, renderer (same with the human mapping):

- `None -> Move`, 12 bytes before (line 35) and 12 bytes after (line 44): the Full painting moves
  the indentation together with `p.skipChildren();` (1:2 Match whose spans include the leading 12
  spaces); omnidiff's Full rendering paints the statement Move but not its indentation.
- `None -> Insert`, 16 bytes, line 41 after: the indentation of the deeper copy, which the Full
  painting splits into 12 moved spaces and 4 inserted ones; omnidiff leaves it unpainted.

`renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: src/diff/text.rs Full painting of the leading indentation of a moved
  statement
- Ground truth questionable: no
