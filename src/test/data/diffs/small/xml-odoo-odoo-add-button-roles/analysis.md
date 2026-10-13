# Accuracy analysis: xml-odoo-odoo-add-button-roles

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 109 (71 visible) of 4740 nodes, 4740 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

Odoo point_of_sale `product_configurator_popup.xml` (an OWL template file): `role="button"` is
added to `<label>` and `<input>` elements in several templates (before lines 17, 47, 97, 132, 157),
and trailing spaces are stripped from a dozen lines. All mismatches are in two near-twin
templates, `point_of_sale.ColorProductAttribute` (before lines 87-113) and
`point_of_sale.ImageProductAttribute` (before lines 147-182), whose `<label>` each gains
`role="button"`.

## Mapping

- Cross-template label pairing, 38 visible, 10 invisible, reason `StructurallyIdenticalSubtrees`
  for the `<label>` element and `StructurallyIdenticalAncestor` for its contents. Before, the
  Color label (line 97) has five attributes (`t-attf-class`, `t-att-class`, `t-attf-style`,
  `t-att-data-color`, `t-att-title`); after, the Image label (after line 159) also has five
  (`role` plus its four). Adding `role="button"` made the two structurally identical, so omnidiff
  pairs the before Color label with the after Image label and shifts every attribute by one:
  `t-attf-class` -> `role`, `t-att-class` -> `t-attf-class`, ... (before lines 97-99, its `</label>`
  on line 108 and two whitespace nodes; after line 160's `role="button"` shows the same from the
  after side). The ground truth keeps each label in its own template. `matcher`.
- The Image label left behind, 26 visible, 6 invisible (before lines 157-170). Its real counterpart
  is taken, so the pieces scatter: `APTED("fast_fallback")` deletes `<label`, the `t-attf-class`
  and `t-attf-style` attributes and `>` (13 visible), and `MovedSubtree` moves its `t-att-class`,
  `t-att-data-color` and `</label>` to the after Color label on lines 99-109 (13 visible), whose
  own before attributes went to the Image label. `matcher`.
- Whitespace between the scrambled elements, 7 visible (before lines 108, 113, 161, 169, 170, 174,
  182): `APTED("fast_fallback")` pairs the `CharData` indentation nodes with gaps in the other
  template or deletes them. `matcher`.
- Enclosing containers, 22 invisible: both templates' `<t>`, `<div>`, `<ul>`, `<li>`, `<span>`
  and the whitespace between them are left `UnresolvedNode`, since their children now map into
  different parents. `matcher`.

Same mechanism as `xml-mozilla-firefox-firefox-add-a-few-translations-and-a-few-attributes`: an
added attribute changes an element's shape, and the structural pass then prefers a
shape-identical element elsewhere over the real counterpart.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: `StructurallyIdenticalSubtrees` (do not pair XML elements across
  different parents when every attribute name would shift; prefer the counterpart under the
  already-matched ancestor)
- Ground truth questionable: no
