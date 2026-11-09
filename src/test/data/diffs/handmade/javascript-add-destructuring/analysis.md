# Accuracy analysis: javascript-add-destructuring

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 19 (19 visible) of 134 nodes, 142 graded, N:M floor 8 visible |
| Painting, Minimal | 83 bytes wrong of 335: renderer 29, matcher 88 |
| Painting, Full | 80 bytes wrong of 335: renderer 36, matcher 84 |
| Ground-truth invariant violations | none |

## The change

Handmade. Three declarations `const name = user.name;` (and `age`, `email`, lines 7-9) become one
destructuring `const { name, age, email } = user;` (line 7). description.md: "Requires N:M
mapping".

## Mapping

- N:M floor, 8 visible. The ground truth puts the three before `const`, `=`, `user` and `;` tokens
  into four all-to-all 3:1 groups with the single after token. Only one member of each group can
  be paired one-to-one; the other two before members of each group (8 nodes) are unavoidable.
  `nm-floor`.
- Group members omnidiff could have paired, 8 visible. For each of the four groups omnidiff pairs
  no member at all: every before token is Delete and every after token is Insert, reason
  `APTED("fast_fallback")`. The three statements and the one new statement are left as unmatched
  roots, and the fallback's Myers-style alignment does not pair any of the before declarations
  with the new one. `matcher`.
- Identifiers into the pattern, 3 visible. Before `name`/`age`/`email` (lines 7-9, col 7, kind
  `identifier`) should map to after's `shorthand_property_identifier_pattern` nodes on line 7.
  omnidiff deletes them, `APTED("fast_fallback")`; the kinds differ and the parents are not
  paired, so nothing carries them over. `matcher`.

## Painting

Matcher-caused (all runs of omnidiff's rendering, 83 bytes Minimal, 80 Full): omnidiff paints
before lines 7-9 entirely Delete and after line 7 entirely Insert. The painting has `const`, the
name and `= user` on each of lines 7-9 as Move (3:1 into after line 7), `.name` / `.age` /
`.email` as Delete, and on after line 7 `const`, the names and `= user` as Move with the new `{`,
`,`, `,`, `}` as Insert. Labels Delete -> Move / None and Insert -> Move / None. `matcher`.

Renderer residual (what stays with the human mapping, 29 bytes Minimal, 36 Full): before `name`,
`age`, `email` (lines 7-9) are Move in the painting but unpainted when the human mapping is
rendered, and on after line 7 the names (Move) and the new `{`, `,`, `}` (Insert) are unpainted.
The human pairs are byte-identical across kinds (`identifier` vs
`shorthand_property_identifier_pattern`), and the renderer paints them nothing rather than Move;
the new punctuation is left unpainted as Minimal punctuation, where this painting marks it
Insert. In Full the same runs widen to include the surrounding spaces. Labels None -> Move /
Insert. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `nm-floor`, `renderer`
- Where a fix would go: `APTED("fast_fallback")` (pair one of the three folded declarations with
  the new one; carry identifiers into the object pattern); renderer: Move for byte-identical pairs
  that changed position inside an N:M fold
- Ground truth questionable: no
