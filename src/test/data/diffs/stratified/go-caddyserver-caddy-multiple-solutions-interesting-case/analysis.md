# Accuracy analysis: go-caddyserver-caddy-multiple-solutions-interesting-case

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 3 (3 visible) of 27873 nodes, 27869 graded, N:M floor 0 visible |
| Painting, Minimal | 15 bytes wrong of 110301: renderer 23, matcher 23 |
| Painting, Full | 16 bytes wrong of 110301: renderer 21, matcher 27 |
| Ground-truth invariant violations | none |

## The change

`slices.Sort[string](defaultLog.Exclude)` becomes `sort.Strings(defaultLog.Exclude)` (line 358).
description.md: "Allows multiple completely different solutions, depending if given node types are
preferred to match or not." The fixture has four paintings: `Minimal` and `Full` (delete and insert)
and `Minimal (Prefer update)` and `Full (Prefer update)` (rename). The benchmark scored Minimal
against `Minimal` and Full against `Full (Prefer update)`.

## Mapping

- **Punctuation across a re-parsed construct (3 visible, `ambiguous`,
  `APTED("large_flat_subtree")`).** tree-sitter-go parses the before `slices.Sort[string](...)` as a
  `type_conversion_expression` (generic type `slices.Sort[string]` applied to a value). The after
  `sort.Strings(...)` is a `call_expression`. The ground truth deletes the before construct and
  inserts the after one, but it carries the selector `.` and the argument parens `(` / `)` across as
  identical. Omnidiff deletes those three tokens too, and it keeps only `defaultLog.Exclude`. The
  parents differ in kind, and the description says this fixture's answer depends on whether such
  node types are preferred to match, so either answer is defensible. The ground truth's tree mapping
  records one of them.

## Painting

The real bytes (Minimal 15, Full 16) are below both the renderer and the matcher shares. Omnidiff's
own mapping renders closer to the paintings than the human mapping does.

- **Matcher-owned (`ambiguous`).** Omnidiff paints the before `.`, `(`, `)` Delete and the after `.`
  Insert. The paintings keep them (Minimal) or match them (Full's `.`).
- **Renderer (Minimal 23, Full 21 bytes).** With the human mapping, the deleted `slices` / `Sort`
  and the inserted `sort` / `Strings` are rendered as Updates narrowed by their shared first letter
  (`s`, `S`): each run's first byte is unpainted. The `Minimal` painting has Delete and Insert.
  `Full (Prefer update)` paints the whole words as Updates. Either way, splitting at a single shared
  first letter is the "shared prefix inside a word" case of text_painting_findings.md rule 3.

## Verdict

- Primary cause: `ambiguous`
- Other causes: `renderer`
- Where a fix would go: none required for the mapping, since the fixture accepts several solutions.
  For the renderer, `src/diff/text.rs` should not narrow an Update by a shared in-word first letter.
- Ground truth questionable: no
