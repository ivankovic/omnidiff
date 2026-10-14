# Accuracy analysis: html-mozilla-firefox-firefox-test-span

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 119 nodes, 121 graded, N:M floor 2 visible |
| Painting, Minimal | 8 bytes wrong of 739: renderer 5, matcher 4 |
| Painting, Full | 20 bytes wrong of 739: renderer 17, matcher 4 |
| Ground-truth invariant violations | 4 (one [12], three [16]) |

## The change

A reftest adds a `#test > span` CSS rule (after lines 15-18) and wraps the second `f` of
`office` in a span: `<div id="test">office</div>` becomes `<div id="test">of<span>f</span>ice</div>`
(line 17 -> 21). description.md: "Requires N:M mapping".

## Mapping

- N:M split of the text (2 visible; `nm-floor`). The ground truth has an all-to-all group: the
  before text `office` matches all three after texts `of`, `f` and `ice`. omnidiff maps `office`
  to `of` (Update) and inserts `f` and `ice` (`APTED("fast_fallback")`). One one-to-one mapping
  cannot reach the other two.

## Painting

- N:M, Minimal 8 bytes / Full 8 bytes (`nm-floor`): with `office` -> `of` as an Update, the
  renderer paints `fice` on before line 17 as Update and the after `f` and `ice` as Insert. The
  Minimal painting leaves all of this text unpainted (only `<span>` / `</span>` are inserted);
  the Full painting moves the before `f` (col 17) into the span. With the human mapping the
  renderer still paints `fice` and `f` as Update (5 bytes), so these runs are about the split
  itself, not the choice of target.
- Ground truth (`ground-truth`): invariant [12] flags that the Minimal painting paints nothing on
  `office` / `ice`, although the tree says the text changed; [16] flags the Minimal and Full
  word-boundary treatment of `office` / `ice`. These are the same bytes as above.
- Full, `</div>` (12 bytes; `renderer`, also with the human mapping): on after line 21 the end
  tag is pushed right by the inserted span and Full paints it Move; the Full painting leaves it
  unpainted.

## Verdict

- Primary cause: `nm-floor`
- Other causes: `ground-truth`, `renderer`
- Where a fix would go: none for the split (one-to-one limit); Full displacement painting of an
  end tag pushed right on the same row in src/diff/text.rs
- Ground truth questionable: yes: invariants [12] and [16] flag the paintings' treatment of
  `office` -> `of` / `f` / `ice` (unpainted in Minimal despite a tree Update; word boundaries)
