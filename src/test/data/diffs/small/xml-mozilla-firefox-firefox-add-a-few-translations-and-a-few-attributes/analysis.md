# Accuracy analysis: xml-mozilla-firefox-firefox-add-a-few-translations-and-a-few-attributes

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 69 (49 visible) of 592 nodes, 592 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

Firefox for Android, app-links `values-hsb/strings.xml` (Upper Sorbian): the root `<resources>`
gains two namespace declarations (line 9), two existing `<string>` elements gain
`tools:ignore="UnusedResources" moz:removedIn="150"` (lines 13 and 18), and seven new commented
`<string>` entries are added at the end (after lines 37-50).

## Mapping

- 48 visible, 20 invisible: two existing strings paired with new ones, reason
  `StructurallyIdenticalSubtrees` for the two `element` roots and `StructurallyIdenticalAncestor`
  for their tags, attribute, text and end tags. Before line 13
  (`mozac_feature_applinks_confirm_dialog_title`) is paired with the new after line 40
  (`..._hide_details`), and before line 18 (`..._confirm_dialog_title_with_app_name`) with the
  new after line 38 (`..._view_details`). The ground truth maps each to its own after line 13 /
  18, which only gained two attributes. Adding the attributes changed the element's shape, so the
  real counterpart is no longer structurally identical, while the new one-attribute strings are;
  the structural pass pairs elements whose `name` value and text differ entirely. All of each
  element's 12 tokens follow, on both sides. `matcher`.
- 1 visible, the whitespace `CharData` at the end of line 36 (`\n` -> `\n  `, it now indents the
  first new comment): paired as in the ground truth but labelled Identical where the ground truth
  says Update (op-only; reason not printed). `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: `StructurallyIdenticalSubtrees` (do not pair XML elements whose
  identifying attribute values all differ, or prefer the element with the same `name` value
  whose attribute list grew)
- Ground truth questionable: no
