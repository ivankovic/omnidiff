# Accuracy analysis: lua-awesomewm-awesome-add-to-table-constructor

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 53 nodes, 53 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 164: renderer 2, matcher 0 |
| Painting, Full | 0 bytes wrong of 164: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

awesome WM's `ruled` module table gets a second field. `client = require("ruled.client");` becomes
`client       = require( "ruled.client"       ),` and `notification = require( "ruled.notification"
),`
is added (lines 2-3). The field separator changes from `;` to `,`.

## Painting

Both runs are the same with the human mapping.

- Minimal, 2 bytes, None -> Delete/Insert: the `;` at before line 2 column 36 and the `,` at after
  line 2 column 50. omnidiff deletes the one and inserts the other. Under Full it paints them, as
  the Full painting does. Minimal sets `structural_punctuation: false`
  (src/diff/text/render_options.rs), which drops any range made only of brackets and separators
  (`STRUCTURAL_PUNCTUATION` includes `;` and `,`). `restore_paired_brackets` brings back only
  brackets whose partner is painted. The Minimal painting keeps this separator change, because it is
  a real edit and not padding around one. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Minimal `structural_punctuation` filter in src/diff/text/render_options.rs:
  keep a separator Delete/Insert when nothing else on that spot changed (here `;` replaced by `,`).
- Ground truth questionable: no
