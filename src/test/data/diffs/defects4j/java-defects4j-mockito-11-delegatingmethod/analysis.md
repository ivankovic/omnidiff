# Accuracy analysis: java-defects4j-mockito-11-delegatingmethod

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 739 nodes, 739 graded, N:M floor 0 visible |
| Painting, Minimal | 48 bytes wrong of 3341: renderer 48, matcher 0 |
| Painting, Full | 74 bytes wrong of 3341: renderer 74, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Defects4J Mockito-11. `DelegatingMethod.equals` gets an identity check and an `instanceof
DelegatingMethod` branch that compares the wrapped methods. The old `return method.equals(o);` moves
into the new `else` branch, with its 12-space indentation unchanged. `hashCode()` returns
`method.hashCode()` instead of `1`.

## Painting

All runs are renderer-owned: rendering the human mapping gives the same bytes.

- **`None -> Move`, line 55 before / line 62 after.** Both paintings carry `return
  method.equals(o);` as a Match: it moved into the new `else` block. omnidiff leaves it unpainted.
  That is 24 bytes per side in Minimal (48), and 36 per side in Full (72), where the Full painting
  also matches the indentation. `identical_or_move` in `src/diff/text.rs` paints a matched node Move
  only when its start column changes (or it crosses backwards). A row-only shift counts as an
  unrelated edit above, and this statement kept its column. `renderer`.
- **Full, `Move -> None`, 2 bytes.** The `;` of `return 1;` (line 60) -> `return method.hashCode();`
  (line 68) only shifted column. Full paints it Move, and the painting leaves it. `renderer`.

The wrap convention differs across fixtures. `java-defects4j-lang-28-numericentityunescaper` paints
the same shape Move, as this fixture does. `java-defects4j-math-95-fdistributionimpl` and
`java-defects4j-math-103-normaldistributionimpl` leave a statement wrapped into a new `if`/`try` at
unchanged indentation unpainted. `java-defects4j-mockito-15-finalmockcandidatefilter` paints it Move
in Full only.

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`
- Where a fix would go: `src/diff/text.rs`: Move painting for a statement wrapped into a new block
  at unchanged indentation; Full Move on a column-shifted identical `;`
- Ground truth questionable: yes, across fixtures: the wrap is painted Move here and in lang-28 but
  left unpainted in math-95 and math-103, and Move in Full only in mockito-15; Marko's call which is
  the convention
