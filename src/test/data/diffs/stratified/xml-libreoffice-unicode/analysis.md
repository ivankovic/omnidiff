# Accuracy analysis: xml-libreoffice-unicode

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 182 nodes, 182 graded, N:M floor 0 visible |
| Painting, Minimal | 18 bytes wrong of 1140: renderer 18, matcher 0 |
| Painting, Full | 18 bytes wrong of 1140: renderer 18, matcher 0 |
| Ground-truth invariant violations | none |

## The change

LibreOffice zh-TW autotext `Q/Q.xml`, line 4: in one `<text:p>`, `感謝您詢問 &lt;placeholder:"November
21, 2004":...&gt;。` becomes `感謝您於 &lt;placeholder:"2004 年 11 月 21 日":...&gt; 的詢問。`: the
date is localised and `詢問` moves behind the placeholder. description.md: "I can't read this, so
I hope this is correct."

## Painting

There are no mapping mismatches; the runs are the same with the human mapping, so all of it is the
renderer's. The single painting is `Minimal` and is scored under both presets.

- Move -> None, the entity reference `&lt;` on line 4, both sides (8 bytes). It is matched and
  unchanged, but its byte column shifts by 3 (`詢問` is 6 bytes, `於` 3), and omnidiff paints it
  Move; the painting leaves it unpainted. The `&gt;`, also shifted, is not painted Move by
  omnidiff; why the two differ was not traced. `renderer`.
- None (Minimal) / Update (Full) -> Insert, ` 的詢問` after `&gt;` on after line 4 (10 bytes). The
  text node after `&gt;` gains this prefix; the painting inserts it, omnidiff's Minimal rendering
  leaves it unpainted and its Full rendering paints it Update. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` (no Move for an in-order node whose column shifted; paint a
  pure prefix insertion into an updated text node as Insert)
- Ground truth questionable: no (the painting reads `詢問` -> `於` as an Update plus an Insert of
  `的詢問` rather than a move of `詢問`; both are defensible, and the mapping agrees with it)
