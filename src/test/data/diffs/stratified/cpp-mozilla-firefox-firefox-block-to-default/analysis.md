# Accuracy analysis: cpp-mozilla-firefox-firefox-block-to-default

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 4278 nodes, 4278 graded, N:M floor 0 visible |
| Painting, Minimal | 12 bytes wrong of 19812: renderer 12, matcher 0 |
| Painting, Full | 0 bytes wrong of 19812: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Six empty destructor bodies `~X() {}` become `~X() = default;` (before lines 170, 193, 218, 249,
286, 318). clang-format wraps four of them onto two lines.

## Painting

- **Deleted empty bodies (Minimal, 12 bytes, `None -> Delete`, `renderer`).** The deleted
  `compound_statement` is just `{}`. Omnidiff's Minimal rendering drops it as structural
  punctuation, so the deletion is not shown at all. The Minimal painting paints each `{}` Delete.
  The runs are the same with the human mapping. When the brackets are the whole deleted construct,
  dropping them hides the change.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: the Minimal structural-punctuation filter in `src/diff/text.rs`. A deleted
  or inserted bracket pair that is the whole construct (an empty body) would be kept.
- Ground truth questionable: no
