# Accuracy analysis: vimscript-neovim-neovim-changes-in-string

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 274 nodes, 274 graded, N:M floor 0 visible |
| Painting, Minimal | 11 bytes wrong of 1910: renderer 11, matcher 0 |
| Painting, Full | 11 bytes wrong of 1910: renderer 11, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Two edits in neovim's sshconfig ftplugin. The revision comment on line 5 changes
(`2025-07-22 (use :hor term #17822)` becomes ``2026-03-11 (`less -I`)``). On line 20 `-I` is
inserted into the MANPAGER string (`less --pattern=` becomes `less -I --pattern=`).

## Painting

The fixture has one painting (`Minimal`), so it is scored under both presets. All runs are the same
with the human mapping, so none is the matcher's.

- Year digit, 4 bytes, Update -> None, line 5 columns 22-23 on both sides (`5-` / `6-`). The year
  changes from 2025 to 2026. omnidiff's narrowed comment Update starts at the first differing
  character, the `5`. The painting's Match starts at column 24, the month, so the changed year digit
  is unpainted. This looks like a slip in the painting. The `-` is unchanged and is painted only
  because it lies between two changes. `ground-truth`.
- Closing parenthesis, 2 bytes, None -> Update, line 5 (before column 46, after column 35). omnidiff
  trims the common suffix `)` from the comment's Update, so the `(` is painted and its `)` is not.
  The painting keeps the pair together (painting rule 2: brackets share fate). `renderer`.
- Inserted option, 5 bytes, after line 20 columns 60-65. The painting inserts ` -I` (columns 60-63).
  omnidiff's narrowed update of the string leaf trims the common prefix `less -` and the common
  suffix `-pattern...`, so it paints the other alignment, `I -` (columns 62-65), and labels it
  Update. Both alignments are textually valid; the painting's keeps the option word whole.
  `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`
- Where a fix would go: narrow Update rendering in src/diff/text.rs: do not trim a closing bracket
  whose opener is inside the range; prefer word-aligned spans; and label a pure insertion inside a
  leaf Insert.
- Ground truth questionable: yes: the Minimal painting's Match for the revision comment starts at
  the month and leaves the changed year digit (2025 -> 2026) unpainted.
