# Accuracy analysis: ruby-mastodon-mastodon-use-context-and-new-test-case

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 8 (3 visible) of 2012 nodes, 2012 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In an RSpec file, the `it 'creates and verifies the item'` example inside
`context 'when item does not yet exist'` (before lines 49-60) is wrapped in a new
`context 'when a position is given' do ... end` and re-indented, and a second new context
`'when no position is given'` is added after it (after lines 50-71).

## Mapping

- **Container chain unmapped** (5 invisible, `UnresolvedNode`): the top-level `body_statement`
  (line 6), `context 'when the collection item is inlined'` (line 44) with its `do_block` and
  `body_statement`, and `context 'when item does not yet exist'` (line 49). These unnamed RSpec
  `call ... do` blocks are not reached by a per-item APTED call, so they are left to the
  residual fallback, which leaves them unresolved.
- **`end` keywords paired by sequence** (3 visible, `APTED("fast_fallback")`). After has three
  more `end`s than before. The LCS fallback (`resolve_residual_forest_via_myers_lcs`) pairs the
  leftover `end`s in order without regard to nesting: before line 86 (`end` of the inlined
  context, column 3) takes after line 71 (column 5, the `end` of the does-not-exist context), and
  before line 115 (the file's last `end`) takes after line 97. The ground truth keeps each at its
  block: before 86 -> after 97, before 115 -> after 126. `matcher`.

## Ground truth

The ground truth maps the `end` of `context 'when item does not yet exist'` (before line 60) to
the `end` of the new inner `context 'when a position is given'` (after line 61), and marks that
context's own `end` on after line 71 as inserted, while it maps the context call itself
(before line 49 -> after line 49). So the `do`/`end` pair of one block is split between two
after blocks: the line-diff reading, not the block reading. This does not change the count here
(omnidiff maps before line 60 the same way and is wrong on after line 71 under either reading),
but it is unlike rule 2 of text_painting_findings.md (paired delimiters share fate). Marko's call.

## Verdict

- Primary cause: `matcher`
- Other causes: `ground-truth`
- Where a fix would go: the residual LCS fallback pairing `end` keywords without nesting, and
  matching unnamed nested `context/it ... do` calls before the fallback.
- Ground truth questionable: yes: the `end` of the kept `context 'when item does not yet exist'`
  block is mapped to the new inner context's `end`, and the block's own `end` is marked inserted.
