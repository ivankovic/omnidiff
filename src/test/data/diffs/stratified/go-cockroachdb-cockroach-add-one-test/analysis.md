# Accuracy analysis: go-cockroachdb-cockroach-add-one-test

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 40542 nodes, 40542 graded, N:M floor 0 visible |
| Painting, Minimal | 1 bytes wrong of 121096: renderer 1, matcher 0 |
| Painting, Full | 0 bytes wrong of 121096: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

A generated test file gets one new test function, `TestReadCommittedLogic_alter_index`, on after
lines 201-207. The signature is wrapped: `(` ends line 201, `) {` is line 203, `}` is line 206.

## Painting

One run, Minimal only, and it is the same with the human mapping, so it is the renderer's.

- `None -> Insert`, after line 206, 1 byte: the `}` that closes the inserted function. Minimal
  splits the multi-row Insert into per-row pieces and drops the rows that are only structural
  punctuation (`) {` on line 203 and `}` on line 206). `restore_paired_brackets`
  (`src/diff/text/render_options.rs`) then restores `) {`, because the partner of its `)` (the `(`
  on line 201) is painted. It checks `}` against the result from before the restore, where its
  partner `{` is not yet covered, so `}` stays dropped. The painting paints all of the new lines,
  and the shipped rule is that brackets share fate. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `restore_paired_brackets` in `src/diff/text/render_options.rs`. It should
  count the ranges it restores as covered, and repeat until nothing changes.
- Ground truth questionable: no
