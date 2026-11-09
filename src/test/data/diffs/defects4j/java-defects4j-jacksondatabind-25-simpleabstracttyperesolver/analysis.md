# Accuracy analysis: java-defects4j-jacksondatabind-25-simpleabstracttyperesolver

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 14 (10 visible) of 1012 nodes, 1012 graded, N:M floor 0 visible |
| Painting, Minimal | 34 bytes wrong of 7123: renderer 4, matcher 38 |
| Painting, Full | 45 bytes wrong of 7123: renderer 7, matcher 38 |
| Ground-truth invariant violations | none |

## The change

Line 80: `return type.narrowBy(dst);` becomes `return
config.getTypeFactory().constructSpecializedType(type, dst);`.

## Mapping

All 14 mismatches are one group on line 80. The ground truth deletes the call `type.narrowBy(..)`
with its `type`, `.`, `narrowBy` and brackets, inserts the new call chain with its own brackets and
the argument `type,`, and keeps only `dst` (moved into the new argument list).

omnidiff (`APTED("qualified_name")`, with `LeafBetweenMatchedNeighbours` for the `.`) maps the
before call onto the after outer call and then pairs leaves across levels: `type` -> `config` and
`narrowBy` -> `getTypeFactory` as Updates, the `.` between them, and the brackets of `narrowBy(dst)`
onto the outer `(type, dst)` (10 visible; the two `method_invocation`s and `argument_list`s are the
4 invisible). The pairs mix the receiver chain of the new call with the old one and share no text,
so the reading is not coherent; the ground truth's replacement is the reasonable one. `matcher`.

## Painting

Matcher-caused (absent with the human mapping):

- Both presets, line 80: `type.narrowBy` painted Update where the painting deletes it (13 bytes),
  `config.getTypeFactory` painted Update where it inserts it (21 bytes). Full adds `(dst)` on the
  before side and the outer `(` and ` dst)` on the after side. Minimal 34 bytes, Full 38. `matcher`.

Renderer-caused (also with the human mapping):

- Minimal, 4 bytes: rendered from the human mapping, the deleted and inserted call's `(` and `)` are
  painted Delete/Insert; the Minimal painting leaves brackets unpainted. In omnidiff's own rendering
  these brackets are mapped and not painted. `renderer`.
- Full, 7 bytes: the Full painting marks `dst` Move (and the space before it Insert); omnidiff
  leaves it unpainted, because `dst);` is the identical tail of the row on both sides (the suffix
  case of `node_untouched_on_its_row`), though `dst` moved into another call. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: APTED fallback (qualified_name): replaced call chain with no shared names
- Ground truth questionable: no
