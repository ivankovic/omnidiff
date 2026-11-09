# Accuracy analysis: css-shadcn-ui-ui-completely-broken-treesitter-parsing

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 71 (71 visible) of 32682 nodes, 32682 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In a Tailwind stylesheet, the commit removes `bg-background` from the `@apply` line of
`.cn-navigation-menu-trigger` (line 824). It moves the `.cn-navigation-menu-link` rule up from
before lines 839-841 to after lines 827-829, and edits the moved rule (`rounded-xl` becomes
`rounded-2xl`, and a variant is appended). As the fixture name says, tree-sitter-css cannot parse
this file.

## Mapping

All 71 mismatches are visible leaves. The ground-truth paths are of the form `["ERROR:4111"]` and
`["-:2057"]`: these are direct children of the root. tree-sitter-css has no rule structure for the
`@apply` lines and the selectors near them. The file is a flat sequence of thousands of root-level
`ERROR` nodes and punctuation leaves.

- Moved rule, 59 visible. The ground truth maps the `.cn-navigation-menu-link` header and `@apply`
  body at before 839-840 to their new place at after 827-828. omnidiff maps the header tokens `cn -
  navigation - menu -` to the `.cn-navigation-menu-indicator` header at after 843 (reason
  `APTED("fast_fallback")`), and deletes `link` and 52 tokens of the edited body (`data`, `[`, `=`,
  `bg`, `roun`, `siz`, ...; op Delete, same reason). The body changed in two places, so its tokens
  have no whole-hash twin, and the flat sequence gives the fallback no container to anchor a move
  on. `parse-error`, with a `matcher` share: the phase 6 Myers pass aligns the root-level sequence
  by position and does not detect the moved block.
- Shifted neighbours, 12 visible. The same positional alignment pairs the
  `.cn-navigation-menu-trigger-icon` header at before 827 with the moved `link` header at after 827.
  It pairs the `indicator` header at before 843 with the `trigger-icon` header at after 831; `cn` is
  reached through `OrphanedLeafUnderMatchedParent`, and the rest is deleted. `parse-error`.

## Verdict

- Primary cause: `parse-error`
- Other causes: `matcher`
- Where a fix would go: upstream grammar (tree-sitter-css cannot parse Tailwind `@apply` content).
  Inside omnidiff, only the phase 6 fallback (`resolve_residual_forest_via_myers_lcs`) could detect
  a moved run of root-level leaves.
- Ground truth questionable: no
