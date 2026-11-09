# Accuracy analysis: python-langflow-ai-langflow-actual-change-of-logic

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 5303 nodes, 5303 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `src/backend/base/langflow/logging/logger.py`, the commit:

- removes the custom `AsyncFileSink` class (before lines 157-174) and its imports;
- adds a `log_rotation` parameter and a block that defaults it from `LANGFLOW_LOG_ROTATION`;
- replaces `sink=AsyncFileSink(log_file) if async_file else log_file` (line 257) with
  `sink=log_file` plus `enqueue=async_file, rotation=log_rotation`.

## Mapping

- **Collapsed conditional loses its `log_file` (1 visible), `matcher`.** The ground truth has a
  multi-map group: the two before `log_file`s of
  `AsyncFileSink(log_file) if async_file else log_file` against the single after `log_file`.
  Exactly one pair is expected. omnidiff matches neither. The detail line does not print what it
  did with the after `log_file` or which pass decided it. The change unwraps a
  `conditional_expression` into one of its own operands, the inverse of a wrap. The pass that
  missed it is undetermined: I only had the group summary line. The N:M floor is 0, so a
  one-to-one mapping can satisfy the group.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: unwrap detection (an expression replaced by one of its own identical
  operands), next to the wrap-growth handling; the pass is undetermined
- Ground truth questionable: no
