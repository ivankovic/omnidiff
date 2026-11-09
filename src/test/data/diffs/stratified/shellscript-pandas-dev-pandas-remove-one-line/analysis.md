# Accuracy analysis: shellscript-pandas-dev-pandas-remove-one-line

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 2 (2 visible) of 1515 nodes, 1515 graded, N:M floor 0 visible |
| Painting, Minimal | 7 bytes wrong of 10550: renderer 2, matcher 5 |
| Painting, Full | 15 bytes wrong of 10550: renderer 2, matcher 13 |
| Ground-truth invariant violations | none |

## The change

The commit removes one `-i "pandas.tseries.offsets.FY5253.rule_code GL08" \` line from the long
argument list of a `validate_docstrings` call in `ci/code_checks.sh`. The whole argument list is one
flat `command` with hundreds of `word`/`string` children (the file's root parses as `ERROR`, but
the error does not touch the changed lines).

## Mapping

- Lines 87-88 (before), 2 visible. The ground truth deletes the `-i` word on the deleted line 87
  together with its string, and maps the `-i` of line 88 to the `-i` of after line 87. omnidiff
  keeps before line 87's `-i` as Identical to after line 87's `-i` and deletes the `-i` of line 88
  instead, both with reason `APTED("large_flat_subtree")`. The flat fast path diffs the children as
  a sequence `-i a -i b -i c` -> `-i a -i c`; deleting `-i b` and deleting `b -i` give the same LCS,
  and the tie-break picked the second, which splits the flag from its argument. All `-i` words are
  identical, so only position decides; the ground truth's choice (flag stays with its string, the
  whole line goes) is the one a reader expects. `matcher`.

## Painting

Matcher-caused (absent with the human mapping):

- Minimal, 5 bytes: `-i ` on line 87 None -> Delete (3) and `-i` on line 88 Delete -> None (2).
- Full, 13 bytes: the same, with line 87's indentation included (`        -i `, 11) plus `-i` on
  line 88 (2).

Renderer-caused (present with the human mapping too), both presets, 2 bytes: the trailing ` \`
line continuation of the deleted line 87 is left unpainted; the painting deletes it with the line.
`renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: tie-breaking of the Myers fast path for flat containers
  (`solve_large_flat_subtrees.rs` / `apted` flat path): prefer deleting a run that starts at a
  line start, so a line's tokens go together; and the renderer's handling of the line-continuation
  backslash on a fully deleted line.
- Ground truth questionable: no
