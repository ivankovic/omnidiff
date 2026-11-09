# Accuracy analysis: c-openssl-openssl-format-only-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 652 nodes, 652 graded, N:M floor 0 visible |
| Painting, Minimal | 275 bytes wrong of 5729: renderer 275, matcher 0 |
| Painting, Full | 641 bytes wrong of 5729: renderer 641, matcher 0 |
| Ground-truth invariant violations | none |

## The change

openssl `der_rsa_sig.c`, whitespace only: the line-continuation backslashes of the
`MD_with_RSA_CASE` macro (lines 27-30) are realigned to a shorter column, and the parameter
`int mdnid)` on line 34 is re-indented from column 55 to column 4. `description.md`: "Whitespace
only. Minimal painting is empty."

## Painting

The mapping is perfect and every run is identical with the human mapping: all of it is the
renderer's.

- Minimal, `Update -> None`, lines 28-30 on both sides, 275 bytes: the macro body is one
  `preproc_arg` token whose text differs only in the whitespace before each `\`. omnidiff maps it as
  an Update and paints the changed part of the token, from the `\` ending line 28 to the `;` on line
  30. The Minimal painting is empty, by design: a whitespace-only change is no change.
  `renderer`.
- Full, `None/Update -> Move`, lines 27-30 on both sides (564 bytes): the Full painting paints
  every realigned macro line as a whole-line Move. omnidiff leaves lines 27-28 unpainted and paints
  lines 29-30 as the Update above. `renderer`.
- Full, line 34 (77 bytes): the Full painting deletes the removed indentation on the before side
  and paints `int mdnid` Move on both sides. omnidiff leaves the reindented parameter unpainted:
  it stays on its row index, and the rest of the row differs only in whitespace. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs` update rendering (`intra_node_update_ranges`) for a
  token whose two texts differ only in whitespace (Minimal), and the Full treatment of reindented
  or realigned unchanged text
- Ground truth questionable: no
