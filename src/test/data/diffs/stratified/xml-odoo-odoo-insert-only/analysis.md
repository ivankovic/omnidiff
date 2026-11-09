# Accuracy analysis: xml-odoo-odoo-insert-only

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 144 nodes, 144 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 441: renderer 0, matcher 0 |
| Painting, Full | 1 bytes wrong of 441: renderer 1, matcher 0 |
| Ground-truth invariant violations | none |

## The change

The commit adds one attribute to an XML element on line 6: `<ShadowOption/>` becomes
`<ShadowOption t-if="!this.excludeShadowOption"/>`. Nothing else changes.

## Painting

- Renderer, Full only, 1 byte, None -> Insert: after line 6, the space between `ShadowOption`
and `t-if` (column 17). The Full painting has a separate one-byte Insert entry for it next to the
attribute Insert (columns 18-50); omnidiff paints only the attribute. The run is the same with the
human mapping, so it is the renderer's. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Full rendering in src/diff/text.rs (extend an inserted node over the
  separating whitespace before it).
- Ground truth questionable: no
