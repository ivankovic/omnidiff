# Accuracy analysis: php-wordpress-wordpress-not-sure-if-this-parses-correctly

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 102084 nodes, 102074 graded, N:M floor 0 visible |
| Painting, Minimal | 48 bytes wrong of 576829: renderer 23, matcher 50 |
| Painting, Full | 51 bytes wrong of 576829: renderer 24, matcher 52 |
| Ground-truth invariant violations | none |

## The change

`wp_post_preview_js()` in `functions.php` gains one line inside its inline `<script>` HTML block:
`//# sourceURL=<?php echo rawurlencode( __FUNCTION__ ); ?>` (after line 7800), just before
`</script>` and the `<?php` that resumes PHP. Marko's `description.md`: "This can't be solved
because of how TreeSitter parses HTML and PHP combined."

The new line splits one HTML text region into two: before, `?>` + text ending in
`</script>\n\t` + `<?php`; after, `?>` + text ending in `//# sourceURL=` + `<?php`, the new
`echo`, then `?>` + `\n\t</script>\n\t` + `<?php`. The old text leaf corresponds to parts of two
new ones. The ground truth inserts only the `echo` statement and leaves 10 nodes ungraded (graded
102074 of 102084), presumably the new tags and text pieces it could not express one-to-one.

## Mapping

- Before line 7801, `<?php`, 1 visible. The ground truth maps it to the `<?php` that still opens
  the unchanged PHP code (after line 7802). omnidiff maps it to the new `<?php` inside the
  `sourceURL` comment (after line 7800, column 16), reason `APTED("qualified_name")`, which keeps
  it with the first after text region. That is the tree-wise natural pairing, but it makes the
  untouched `</script>` / `<?php` lines look changed (below). `matcher`, forced partly by the
  parse.

## Painting

Matcher-caused (absent with the human mapping), 28 bytes Minimal, 30 Full:

- Before line 7800 `</script>` painted Update (9) and before line 7801 `<?php` painted Move (5);
  after lines 7801-7802 `</script>` and `<?php` painted Insert (9 + 5; 10 + 6 with the tab in
  Full). The painting leaves all of them unchanged.

Present with both mappings, 20 bytes Minimal, 21 Full:

- After line 7800, `//# sourceURL=<?php `: the painting marks it Insert. omnidiff paints it Update
  (the old text leaf is mapped to the new one ending in `//# sourceURL=`); the human-mapping
  rendering leaves it unpainted, as it does the ` ?>` (3 bytes, which omnidiff's own rendering
  paints Insert correctly). The human mapping does not list these nodes, so its rendering has
  nothing to paint them with; the gap belongs to the leaf granularity of HTML text in the parse,
  not to a single renderer rule. `renderer` (owner), rooted in the parse.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: APTED within the matched function (`qualified_name`) for the `<?php`
  pairing; painting part of an HTML text leaf as inserted needs text-level diffing of `text`
  leaves in `diff::text`. Marko considers the fixture not fully solvable with this parse.
- Ground truth questionable: no (it is deliberately partial: 10 nodes ungraded)
