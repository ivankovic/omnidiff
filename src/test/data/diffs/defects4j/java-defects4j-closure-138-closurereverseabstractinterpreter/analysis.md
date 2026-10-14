# Accuracy analysis: java-defects4j-closure-138-closurereverseabstractinterpreter

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 3 (3 visible) of 3768 nodes, 3768 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 16925: renderer 2, matcher 2 |
| Painting, Full | 0 bytes wrong of 16925: renderer 27, matcher 27 |
| Ground-truth invariant violations | 2 |

## The change

Closure-138: `ClosureReverseAbstractInterpreter`'s `goog.isDef`-style refinement no longer
guards on `paramType != null`: the `if (paramType != null) {` line (before 208) and its closing
`}` (before 218) are removed and the body is dedented.

## Mapping

- The closing braces after the removed `if`, 3 visible (before lines 220-222). The ground
  truth's tree mapping deletes the `{` of the removed `if` (line 208) but keeps a `}` on line
  220, and keeps the enclosing block's `{` (line 203) but deletes its `}` on line 222: each
  closing brace is matched one level off. invariants.txt flags both pairs (rule 3: a bracket
  matched while its partner is deleted). omnidiff deletes the `}` that belongs to the removed
  `if` and keeps the others with their partners (op Identical / Delete,
  `APTED("qualified_name")`). Both of the ground truth's paintings agree with omnidiff: they
  paint the `}` on before line 220 Delete and leave lines 221-222 alone. Rendering the human tree
  mapping is what disagrees with the paintings (2 Minimal / 27 Full renderer bytes).
  `ground-truth`.

## Painting

No disagreement with omnidiff's mapping under either preset (0 real bytes). The renderer bytes
are the human tree mapping rendered against its own paintings, described above.

## Verdict

- Primary cause: `ground-truth`
- Other causes: none
- Where a fix would go: ground truth (Marko's call); nothing in the engine.
- Ground truth questionable: yes: invariant 3 twice; the tree mapping pairs the closing braces on
  before lines 220 and 222 against their opening braces' fate, while both paintings and omnidiff
  delete the removed `if`'s own `}`.
