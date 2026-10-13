# Accuracy analysis: java-defects4j-compress-11-archivestreamfactory

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 2816 nodes, 2816 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 20210: renderer 2, matcher 2 |
| Painting, Full | 0 bytes wrong of 20210: renderer 15, matcher 15 |
| Ground-truth invariant violations | 2 |

## The change

The TAR auto-detection `try { ... } catch (Exception e) { ... }` (lines 240-247 before) is wrapped
in a new `if (signatureLength >= 512) { ... }` (lines 240 and 250 after), without re-indenting the
body.

## Mapping

- Line 249 `}` (before, closes the outer `try` block, `        } catch (IOException e) {`): the
  ground truth maps it to the `}` of the new `if` block (line 250 after, 12-space indent) and marks
  the outer `try` block's own `}` on line 251 as inserted. omnidiff (`APTED("qualified_name")`) maps
  it Identical to line 251's `}`, the brace of the same `try` block. 2 visible (the before brace and
  the after line-251 brace).

omnidiff's answer is the structural one and agrees with both paintings, which insert line 250's `}`.
The ground truth's own invariant check flags the mapping: after `{` on line 210 is matched while its
`}` on line 251 is inserted, and the new `{` on line 240 is inserted while its `}` on line 250 is
matched. `ground-truth`.

## Painting

omnidiff's rendering matches both paintings exactly. Only the HUMAN mapping, rendered, disagrees: it
leaves the new `}` on line 250 unpainted and paints line 251's `}` Insert (2 bytes Minimal; 15 bytes
Full, which also paints the before `}` Move). This is the same ground-truth inconsistency, seen
through the renderer; no reader of omnidiff's output sees it.

## Verdict

- Primary cause: `ground-truth`
- Other causes: none
- Where a fix would go: ground truth (Marko's call); nothing in the engine
- Ground truth questionable: yes: human mapping maps before line 249 } (outer try block) to the new
  if block } on line 250; invariants report both bracket pairs split, and both paintings insert line
  250 }
