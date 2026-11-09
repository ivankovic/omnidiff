# Accuracy analysis: shellscript-langchain-ai-langchain-some-interesting-raw-string-to-string-content

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 183 nodes, 183 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `lint_imports.sh` the two `git --no-pager grep '...' . && errors=$((errors+1))` lines switch
from single-quoted (raw) strings to double-quoted strings, and the first gets two
`| grep -v ":from langchain\.agents"` / `... tools` filters in front of `&&` (after line 10). A
comment line is added.

## Mapping

- Before line 9, the `&&` of `git ... grep '^from langchain\.' . && errors=$((errors+1))`. The
  ground truth maps it to the `&&` on after line 10. Omnidiff deletes it (reason
  `APTED("fast_fallback")`), so a reader sees one `&&` deleted and one inserted on a line whose
  `&& errors=$((errors+1))` tail did not change.
- Why: with the new pipes, tree-sitter-bash parses the after line as a `pipeline` whose last stage
  is the `list` `grep -v ... && errors=...`; the before line is a plain `list`. The `&&` therefore
  changes parent chain (`list` -> `pipeline`/`list`), and the residual LCS fallback, which handles
  this top-level shell code, does not keep it. Which ancestor pairs blocked it was not determined
  from the evidence. The ground truth is the reasonable reading. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: the residual LCS fallback (`resolve_residual_forest_via_myers_lcs`) for
  top-level shell statements whose parse nests deeper after a pipe is added.
- Ground truth questionable: no
