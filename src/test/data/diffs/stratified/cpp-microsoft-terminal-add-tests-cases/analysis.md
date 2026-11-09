# Accuracy analysis: cpp-microsoft-terminal-add-tests-cases

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 5350 nodes, 5350 graded, N:M floor 0 visible |
| Painting, Minimal | 179 bytes wrong of 68975: renderer 179, matcher 0 |
| Painting, Full | 142 bytes wrong of 68975: renderer 142, matcher 0 |
| Ground-truth invariant violations | 6 |

## The change

The commit adds three `TEST_METHOD(...)` declarations to the test class (after lines 48-50) and the
three test functions themselves (after lines 636-937). The functions are mostly large JSON settings
in raw string literals.

## Painting

- **Which TEST_METHOD line is new (Minimal 124, Full 142 bytes, `parse-error`).** tree-sitter-cpp
  does not understand the `BEGIN_TEST_CLASS` / `TEST_METHOD(...)` macros. It parses the list as one
  `field_declaration` with a `parenthesized_declarator` and an `ERROR` node. Each `TEST_METHOD`
  argument becomes a `type_identifier` in a parameter list, and the shape differs between the sides.
  The human tree mapping follows that tree. It updates the last entry,
  `DontRoundtripNoReloadEnvVars` (line 46), into the last new entry,
  `RoundtripUserDeletedColorSchemeCollision` (after line 50), and makes after line 46 new.
  Omnidiff's mapping is the same (0 mismatches). Its rendering therefore paints before line 46
  Update and after line 46 Insert. Both paintings instead keep line 46 unchanged and insert line 50.
  These runs are identical with the human mapping. `invariants.txt` reports the conflict as 6
  violations ([11], [22], [16] for each painting).
- **Bracket-only lines of the inserted tests (Minimal, 55 bytes, `None -> Insert`, `renderer`).**
  The inserted functions' `{`/`}` and the JSON's `{`, `},`, `[`, `]` inside the raw string literals
  stand on their own lines. Omnidiff's Minimal punctuation filter drops them. The Minimal painting
  inserts them with the rest. Inside a string literal they are content, not structure.

## Verdict

- Primary cause: `parse-error`
- Other causes: `ground-truth`, `renderer`
- Where a fix would go: the line 46/50 disagreement cannot be fixed in the matcher while the tree is
  an `ERROR` parse (Marko's call whether the mapping or the painting should give way). The Minimal
  punctuation filter in `src/diff/text.rs` should keep brackets of wholly inserted constructs and of
  string-literal content.
- Ground truth questionable: yes. The tree mapping and both paintings disagree on which
  `TEST_METHOD` line is new (6 invariant violations). This is forced by the `ERROR` parse.
