# Accuracy analysis: java-defects4j-closure-109-jsdocinfoparser

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 3 (2 visible) of 32149 nodes, 32152 graded, N:M floor 2 visible |
| Painting, Minimal | 34 bytes wrong of 158450: renderer 27, matcher 8 |
| Painting, Full | 32 bytes wrong of 158450: renderer 25, matcher 14 |
| Ground-truth invariant violations | 2 |

## The change

Closure-109: `JsDocInfoParser.parseContextTypeExpression` accepts `?`: the single
`return parseTypeName(token);` (before line 1908) becomes
`if (token == JsDocToken.QMARK) { return newNode(Token.QMARK); } else { return
parseBasicTypeExpression(token); }` (after lines 1908-1912).

## Mapping

- The ground truth matches the one before `return` statement to both after returns (1:2, an
  all-to-all group): `return` and `;` on after lines 1909 and 1911. omnidiff, one-to-one, maps it
  to the `else` branch's return (line 1911) and inserts the `then` branch's return on line 1909
  (op Insert, `APTED("large_flat_subtree")`). The 2 visible mismatches equal the N:M floor.
  `nm-floor` (2 visible, 1 invisible).

## Painting

- Matcher, N:M, Minimal 8 / Full 14 bytes: after line 1909 `return ` (with indentation in Full)
  and `;` are painted Insert where the painting has Move (the 1:2 match). `nm-floor`.
- Renderer, Minimal 27 bytes (survive with the human mapping):
  - `(token);` on before 1908 / after 1911, None -> Move, 16 bytes: the painting treats the
    whole statement as moved into the `else` block and paints each kept token Move. omnidiff
    paints the kept `return` Move but leaves the identical tail `(token);` unpainted. `renderer`.
  - `parse` of `parseTypeName` -> `parseBasicTypeExpression`, None -> Update, 10 bytes: the
    painting paints the whole identifier; the renderer narrows the update to the differing words
    and leaves `parse` alone. invariants.txt flags exactly these two spans of the Minimal painting
    (rule 16: the differing words are `TypeName` / `BasicTypeExpression`). `ground-truth`.
  - after line 1909 `;`, 1 byte: with the human mapping, too, the renderer does not paint the
    second target of the 1:2 match. Part of the N:M group above.
- Renderer, Full 25 bytes: indentation and the space after `return` (before 1908: 4 Delete +
  6 Move; after 1909 and 1911: 6 + 1 Move each), which the Full painting assigns to the deleted
  outer indentation and the moved statement, left unpainted by omnidiff. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `nm-floor`, `ground-truth`
- Where a fix would go: `src/diff/text.rs`: painting an identical tail of a statement that moved
  into a new block, and Full's whitespace of moved/deleted statements.
- Ground truth questionable: yes: invariant 16 flags the Minimal painting of `parse` in the
  renamed method call (before 1908, after 1911).
