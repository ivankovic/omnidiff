# Accuracy analysis: xml-microsoft-terminal-add-one-element

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 1 (1 visible) of 784 nodes, 784 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 2878: renderer 0, matcher 0 |
| Painting, Full | 2 bytes wrong of 2878: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Windows Terminal `tools/tests.xml`: one `<test name="winconpty" ... />` is appended as the last
child of the root (after line 18). The whitespace after line 17's element grows from `\r\n` to
`\r\n  ` (it now indents the new element), and a new `\r\n` follows the new element.

## Mapping

1 visible, the whitespace `CharData` at the end of line 17. omnidiff pairs it with the same node as
the ground truth (`CharData:16` on both sides), but labels the pair Identical where the ground
truth says Update, though the text differs by two spaces. The pairing reason is not printed for an
op-only mismatch; why the whitespace difference is classed Identical was not traced. `matcher`.

## Painting

Full only, None -> Insert, the new element's two-space indentation on after line 18 (2 bytes). The
Full painting inserts it; omnidiff leaves it unpainted, and does so with the human mapping (Update)
as well, so this is the renderer's, not a consequence of the op label. Minimal trims the
indentation and agrees. `renderer`.

## Verdict

- Primary cause: `renderer` (the one visible mapping mismatch is an op label on a whitespace node;
  what a reader sees, the 2 bytes, survives with the human mapping)
- Other causes: `matcher`
- Where a fix would go: `diff::text` Full rendering of the grown whitespace node; the op
  classification of whitespace-only text changes
- Ground truth questionable: no
