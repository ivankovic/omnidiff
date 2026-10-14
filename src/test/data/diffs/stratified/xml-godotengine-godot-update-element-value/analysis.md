# Accuracy analysis: xml-godotengine-godot-update-element-value

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 10204 nodes, 10204 graded, N:M floor 0 visible |
| Painting, Minimal | 254 bytes wrong of 77893: renderer 254, matcher 0 |
| Painting, Full | 258 bytes wrong of 77893: renderer 258, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Godot `doc/classes/Input.xml`: one line of text, `[b]Note:[/b] The order of connected joypads
...`, is appended to the `<description>` of a method (after line 88). In the XML tree the whole
description is one `CharData` text node, so the node is updated rather than a node inserted.

## Painting

There are no mapping mismatches; the runs are identical with the human mapping, so all of it is
the renderer's.

- Minimal, Update -> Insert, the new line's text on after line 88 (254 bytes). The painting
  marks the added line Insert. omnidiff renders the changed `CharData` as an Update over the added
  text, although the before text is an unchanged prefix of the after text and the difference is a
  pure insertion. `renderer`.
- Full, None -> Insert, the same line including its four leading tabs (258 bytes). omnidiff's Full
  rendering does not paint the run as Insert either (it is unpainted at the run's first byte, the
  indentation). `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` narrowing of an updated text leaf (paint a pure appended
  substring Insert, not Update)
- Ground truth questionable: no
