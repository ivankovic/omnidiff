# Accuracy analysis: java-defects4j-jsoup-52-xmldeclaration

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 23 (16 visible) of 829 nodes, 840 graded, N:M floor 15 visible |
| Painting, Minimal | 197 bytes wrong of 3934: renderer 51, matcher 150 |
| Painting, Full | 305 bytes wrong of 3934: renderer 169, matcher 208 |
| Ground-truth invariant violations | none |

## The change

`getWholeDeclaration()` (lines 46-61) shrinks to `return attributes.html().trim();`, and
`outerHtmlHead` (lines 65-69) splits its one `accum.append("<").append(ternary)
.append(getWholeDeclaration()).append(">");` chain into two statements around a new
`attributes.html(accum, out);` (lines 50-56 after): the first ends in `.append(name);`, the second
repeats `accum` and `.append(isProcessingInstruction ? "!" : "?")` and ends with `.append(">");`.
Marko's `description.md` notes the invisible `method_invocation` nodes combine N:M and 1:1 group
mapping.

## Mapping

- N:M, 15 of the 16 visible and the 7 invisible: the ground truth maps the before chain statement
  (line 65) to both after statements in an all-to-all group, with the ternary `.append(..)` and
  `accum` copied 1:2. The listed after nodes are the second statement's `accum`, the copied
  `.append(isProcessingInstruction ? "!" : "?")` leaves (line 56) and the first statement's new `;`
  (line 53). omnidiff (`APTED("qualified_name")`) maps the before statement's parts one-to-one into
  the first after statement and inserts the second. A one-to-one mapping cannot map one `accum` to
  two; the N:M floor is 15 visible nodes. `nm-floor`.
- 1 visible beyond the floor, the `.` of `.append(">")` (line 69): the ground truth maps it to the
  `.` of the second statement's `.append(">")` (line 57); omnidiff deletes it, and the whole
  `.append(">")`, because it put the rest of the chain in the first statement. `matcher`.

## Painting

Both paintings mark every surviving piece of the old chain Move (Minimal 10 entries, two of them 1:2
copies).

- N:M-driven, both presets, Minimal 147 bytes: `accum` and the ternary `.append(..)` on lines 65 and
  67 before and lines 50, 52, 55, 56 after. With the human (N:M) mapping these render Move;
  omnidiff's one-to-one mapping leaves the first copy in place (unpainted) and inserts the second.
  `nm-floor`.
- Matcher, both presets: `.append(">");` (line 69 / 57, 13 bytes each side) painted Delete and
  Insert where the painting moves it. `matcher`.
- Renderer (also with the human mapping), Minimal 51 bytes: `.append("<")` (line 66 / 51) and
  `.append(">");` are left unpainted although the paintings mark them Move: they keep their columns
  and only shift rows (15 rows up), which the renderer does not count as a move; and the new `;` on
  line 53 is left out of the Insert. `renderer`.
- Full adds the indentation of each of these lines, which the Full painting moves along with them
  (as in `java-defects4j-jacksondatabind-39-nullifyingdeserializer`). `renderer`.

## Verdict

- Primary cause: `nm-floor`
- Other causes: `matcher`, `renderer`
- Where a fix would go: none for the N:M share (one-to-one mapping); APTED fallback (qualified_name)
  for the split chain; renderer row-only-shift move rule
- Ground truth questionable: no
